while true {
  print $"(cat /sys/class/power_supply/BAT*/capacity).0"
  sleep 10000ms
}
