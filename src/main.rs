use anyhow::Result;
use hyprland::{
    data::{Client, Clients, Workspace},
    event_listener::EventListener,
    prelude::*,
};
use lazy_static::lazy_static;
use psutil::process::Process;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};
use tokio::spawn;
use tokio::time::sleep;

type WorkSpaceInfo = BTreeMap<i32, IconSet>;

#[derive(Debug, Clone, Deserialize)]
struct Config {
    icons: BTreeMap<String, RawIcon>,
}

#[derive(Debug, Clone, Deserialize, Ord, PartialEq, Eq, PartialOrd)]
struct RawIcon {
    gui: Option<Vec<String>>,
    tui: Option<Vec<String>>,
    ord: Option<u32>,
    font_size: Option<String>,
    other: Option<bool>,
    active: Option<bool>,
    empty: Option<bool>,
}

#[derive(Debug, Clone,  Ord, PartialEq, Eq, PartialOrd)]
struct Icon {
    icon: &'static str,
    gui: &'static [&'static str],
    tui: &'static [&'static str],
    ord: u32,
    font_size: &'static str,
    other: bool,
    active: bool,
    empty: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let listener = Box::leak(Box::new(EventListener::new()));
    listener.add_active_monitor_change_handler(|_| on_window_event());
    listener.add_window_close_handler(|_| on_window_event());
    listener.add_window_moved_handler(|_| on_window_event());
    listener.add_window_open_handler(|_| on_window_event());

    listener.add_workspace_added_handler(|_| on_window_event());
    listener.add_workspace_change_handler(|_| on_window_event());
    listener.add_workspace_destroy_handler(|_| on_window_event());

    let lisntner_handle = spawn(listener.start_listener_async());
    let update_handle = spawn(update());

    lisntner_handle.await??;
    update_handle.await?;
    Ok(())
}

fn find_nested_programs_in_termainals(
    terminal_pids: &[i32],
    terminal_workspaces: &[i32],
) -> Result<BTreeMap<i32, IconSet>> {
    let processes = psutil::process::processes()?;
    let processes: Vec<&Process> = processes.iter().flatten().collect();
    let mut t = BTreeMap::new();
    for process in &processes {
        if let Ok(name) = process.name() {
            if let Some(icon) = NAME_TO_ICON.get(name.as_str()) {
                if let Some(idx) = is_running_from_terminal(terminal_pids, process) {
                    t.entry(terminal_workspaces[idx])
                        .or_insert(BTreeSet::new())
                        .insert(icon.clone());
                }
            }
        }
    }
    Ok(t)
}

fn is_running_from_terminal(terminal_pids: &[i32], process: &Process) -> Option<usize> {
    match terminal_pids.binary_search(&(process.pid() as i32)) {
        Ok(i) => Some(i),
        _ => match process.parent() {
            Ok(Some(p)) => is_running_from_terminal(terminal_pids, &p),
            _ => None,
        },
    }
}

async fn update() {
    loop {
        sleep(Duration::from_millis(5000)).await;
        on_window_event();
    }
}

fn on_window_event() {
    let active_workspace = match Workspace::get_active().ok() {
        Some(id) => id,
        None => return,
    }
    .id;
    render(active_workspace)
        .err()
        .map(|e| eprintln!("{:#?}", e));
}


fn render(id: i32) -> Result<()> {
    let clients = Clients::get()?.to_vec();
    let mut terminal_info = clients
        .iter()
        .filter_map(|c| {
            if c.class == "foot" && c.pid != -1 {
                Some((c.pid, c.workspace.id))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    terminal_info.sort_by_key(|item| item.0);
    let terminal_pids = terminal_info.iter().map(|i| i.0).collect::<Vec<_>>();
    let terminal_workspaces = terminal_info.iter().map(|i| i.1).collect::<Vec<_>>();
    let mut workspace_icons = get_workspace_icons(&clients);
    let nested = find_nested_programs_in_termainals(&terminal_pids, &terminal_workspaces)?;
    for (id, icons) in nested.iter() {
        let set = workspace_icons.entry(*id).or_insert(BTreeSet::new());
        for icon in icons {
            set.insert(icon.clone());
        }
    }

    let yuck_code = render_workspaces_yuck(id, &workspace_icons);
    // Eww listens to std out
    println!("{yuck_code}");
    Ok(())
}

type IconSet = BTreeSet<&'static Icon>;

fn get_workspace_icons(clients: &[Client]) -> BTreeMap<i32, IconSet> {
    let workspace_icons = clients
        .iter()
        .filter(|c| c.pid != -1) // Windows that are animating out report pid -1
        .fold(BTreeMap::new(), |mut acc, client| {
            let id = client.workspace.id;
            let ws = acc.entry(id).or_insert(BTreeSet::new());
            let icon = CLASS_TO_ICON.get(client.class.as_str()).unwrap_or(&OTHER_ICON);
            ws.insert(icon.clone());
            acc
        });
    workspace_icons
}

fn render_workspaces_yuck(active_workspace_id: i32, workspace_info: &WorkSpaceInfo) -> String {
    let buttons = (1..=10)
        .map(|id| {
            let is_active = active_workspace_id == id;
            let color = color(is_active, workspace_info, id);
            let icon = icon(is_active, &workspace_info.get(&id));
            let size = &icon.font_size;
            render_button(id, &icon.icon, &format!("color: {color};font-size: {size}"))
        })
        .collect::<String>();

    format!("(box :valign 'start' :halign 'start' :orientation \"v\" {buttons} )")
}
type WorkspacesToIcons = BTreeMap<i32, IconSet>;
fn color(is_active: bool, workspace_info: &WorkspacesToIcons, id: i32) -> &str {
    let color = if is_active {
        "#fb4934"
    } else if workspace_info.contains_key(&id) {
        "#83a598"
    } else {
        "#fabd2f"
    };
    color
}

fn icon(is_active: bool, info: &Option<&IconSet>) -> &'static Icon{
        info.map_or_else(
            || if is_active {
                &*ACTIVE_ICON
            } else {
                &*EMPTY_ICON
            },
            |set| set
                .iter()
                .map(|icon| (icon, icon.ord))
                .max_by_key(|(_i, o)| o.clone())
                .map(|(i, _o)| i)
                .unwrap_or(&OTHER_ICON)
        )
}

fn render_button(id: i32, icon: &str, style: &str) -> String {
    format!("(button :class 'workspace' :style '{style}' :onclick 'hyprctl dispatch workspace {id}' '{icon}')")
}

fn leak<T: ?Sized>(data: Box<T>) -> &'static T{
    let data: &_ = Box::leak(data);
    data
}


lazy_static! {

    static ref CONFIG: &'static BTreeMap<&'static str, &'static Icon> = {
        let config: Config = toml::from_str(&std::fs::read_to_string("./icons.toml").unwrap()).unwrap();
        let mut map = BTreeMap::new();
        let leak_slice = |opt: Option<Vec<String>>|{
             leak(          
                opt.unwrap_or_default().into_iter().map(|string| {
                leak(string.into_boxed_str())
                }).collect::<Vec<_>>().into_boxed_slice()
            )
        };
        for (text, raw_icon) in config.icons.into_iter(){
            let static_text = leak(text.into_boxed_str());
            let icon = Icon{
                    icon: static_text,
                    gui:  leak_slice(raw_icon.gui),
                    tui: leak_slice(raw_icon.tui),
                    font_size: leak(raw_icon.font_size.unwrap_or("1.4rem".to_string()).into_boxed_str()),
                    ord:raw_icon.ord.unwrap_or_default(),
                    other:raw_icon.other.unwrap_or_default(),
                    active:raw_icon.active.unwrap_or_default(),
                    empty:raw_icon.empty.unwrap_or_default(),
            };
            map.insert(static_text,leak(Box::new(icon)))  ;
        };
        leak(Box::new(map))
    };

    static ref CLASS_TO_ICON: BTreeMap<&'static str, &'static Icon> = CONFIG.into_iter().flat_map(|(_text,icon)|
        icon.gui.into_iter().map(move |class| (*class, *icon))
    ).collect();

    static ref NAME_TO_ICON: BTreeMap<&'static str, &'static Icon> = CONFIG.into_iter().flat_map(|(_text,icon)|
        icon.tui.into_iter().map(move |name| (*name, *icon))
    ).collect();

    static ref OTHER_ICON: &'static Icon = CONFIG
        .iter()
        .find_map(|(_, metadata)| metadata.other.then_some(metadata))
        .expect("Set at least one icon to be default");

    static ref ACTIVE_ICON: &'static Icon = CONFIG
        .iter()
        .find_map(|(_, metadata)| metadata.active.then_some(metadata))
        .expect("Set at least one icon to be default");

    static ref EMPTY_ICON: &'static Icon = CONFIG
        .iter()
        .find_map(|(_, metadata)| metadata.empty.then_some(metadata))
        .expect("Set at least one icon to be default");
        
}
