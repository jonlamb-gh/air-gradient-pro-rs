# air-gradient CLI

TODO 

switch to async

## InfluxDB 2

use this crate https://crates.io/crates/influxdb2

See https://portal.influxdata.com/downloads/

```
wget https://dl.influxdata.com/influxdb/releases/influxdb2-2.6.1-linux-amd64.tar.gz
tar xvfz influxdb2-2.6.1-linux-amd64.tar.gz

...

add to $PATH
```

example `~/.config/systemd/user/influxd.service`

```
[Unit]
Description=InfluxDB 2 daemon service

[Service]
Type=simple
LimitNOFILE=65536
ExecStart=influxd --reporting-disabled

[Install]
WantedBy=default.target
```

```
systemctl --user daemon-reload
systemctl --user enable influxd.service
systemctl --user start influxd.service
```

Default address is `http://localhost:8086`
