# P2proxyd

A peer-to-peer TCP daemon.

## Usage

Run the proxy with:

`./p2proxyd run --cfg-path <path>`

Or spawn it with:

`nohup ./p2proxyd run --cfg-path <path>&`

Or use it with `systemd`:

```unit file (systemd)
[Unit]
Description="P2proxyd service"
After=network.target

[Service]
Type=simple
Restart=always
RestartSec=1
User=<user>
ExecStart=/<bin-path>/p2proxyd run --cfg-path <cfg-path>

[Install]
WantedBy=multi-user.target
```

## Configuration

There are 3 components to configuration.

### Identity

The server device has a secret key, and that secret key's corresponding public key is used for routing (as an address).

### Targets

A target is a port, or an ip+port, optionally with a name for routing.

### Access

Which nodes can access which routes.

## Examples

The simplest example is this:

```toml
# Node id: 7003b83df94765d4862185187508055e11be90d761663061e4f368d076b7a9b8
secret_key_hex = "11701920da9f96a52625963997db5bc54e27ea86b00094646e2e860c4a8fa796"
allow_any_peer = true
default_route = "default"

[[server_ports]]
port = 8080
name = "default"
allow_any_peer = true
```

It will forward any incoming requests to port 0.0.0.0:8080, that means that anyone who knows the node id:
`7003b83df94765d4862185187508055e11be90d761663061e4f368d076b7a9b8` can connect. This may be a security concern,
depending on what's running on port 8080.

### Full configuration example

```toml
# Node id: 7003b83df94765d4862185187508055e11be90d761663061e4f368d076b7a9b8
secret_key_hex = "11701920da9f96a52625963997db5bc54e27ea86b00094646e2e860c4a8fa796"
# Use an access log, writes down accepted and rejected connections, with cause
# If using, the log *should* be rotated with f.e. `logrotate`
access_log_path = "/home/<user>/logs/p2proxy-access.log"
# If no named port is specified, fall back to this route
default_route = "demo"

# A collection of exposed services through the proxy
[[server_ports]]
# Ip, if for example there's an entry-server that runs the proxy to some other server on a local network.
host_ip = "192.168.0.3"
# Port to listen on
port = 4500
# Port name for routing
name = "demo"
# Allow any peer to connect to this service
allow_any_peer = true

[[server_ports]]
# Only a port specified, uses `0.0.0.0:8080`
port = 8080
# Port name for routing
name = "private"

# A collection of approved peers
[[peers]]
# The peer node's node_id
node_id = "7003b83df94765d4862185187508055e11be90d761663061e4f368d076b7a9b7"
# If the peer node can access anything (superuser)
allow_any_port = false
# The ports that this peer is allowed to reach ("demo" has no access control, so it can reach that one as well)
allow_named_ports = ["private"]

[[peers]]
node_id = "7003b83df94765d4862185187508055e11be90d761663061e4f368d076b7a9b6"
# Superuser, can access any service
allow_any_port = true

```
