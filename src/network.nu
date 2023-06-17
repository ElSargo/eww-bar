while true {
  let connections = (nmcli d | jc --nmcli | from json | where state == "connected")
  if ($connections | any { |connection| $connection.type == "ethernet"}) {
    print "󰈀"
  } else if ($connections | any { |connection| $connection.type == "wifi"}) {
    print "󰖩"
  } else {
    print "󱛅"
  } 
  sleep 5000ms
}
