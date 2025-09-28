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
# Node id: 7c32ab7cdd9a4e2651c9eff072958a43a9b411cc5b69603a1dca6d7d843f2406
secret_key_hex = "8c3981f6f98d0a09f69931549a883d8ce1c37fbf767c28ace12c81ede4713bfc"
default_route = "default"

[[server_ports]]
port = 4501
name = "default"
allow_any_peer = true
```

It will forward any incoming requests to port 0.0.0.0:8080, that means that anyone who knows the node id:
`7003b83df94765d4862185187508055e11be90d761663061e4f368d076b7a9b8` can connect. This may be a security concern,
depending on what's running on port 8080.

### Full configuration example

```toml
# Node id: f2b1ce018dda1d4e75d97fc9f86ecf30adbb0aba0977445ae85283502a8cc7be
secret_key_hex = "690927f498c370cff79be198b1e6b81e3ec12521d1a76753c8aff67a7bb6f549"
# Use an access log, writes down accepted and rejected connections, with cause
# If using, the log *should* be rotated with f.e. `logrotate`
access_log_path = "/home/<user>/logs/p2proxy-access.log"
# If no named port is specified, fall back to this route
default_route = "demo"

# A collection of exposed services through the proxy
[[server_ports]]
# Ip, if for example there's an entry-server that runs the proxy to some other server on a local network.
host_ip = "127.0.0.1"
# Port to listen on
port = 4502
# Port name for routing
name = "demo"
# Allow any peer to connect to this service
allow_any_peer = true

[[server_ports]]
# Only a port specified, uses `0.0.0.0:4502`
port = 4503
# Port name for routing
name = "private"

# A collection of approved peers
[[peers]]
# The peer node's node_id (assets/testing-has-access-private-client.key)
node_id = "69a0507ed92bf714b99135024a15628ad508a90db9e142a8518e7a9d939de7ba"
# If the peer node can access anything (superuser)
allow_any_port = false
# The ports that this peer is allowed to reach ("demo" has no access control, so it can reach that one as well)
allow_named_ports = ["private"]

[[peers]]
# Superuser, can access any service (assets/testing/superuser-client.key)
node_id = "7d835f80eb895097e3b1a3648dee0e40e30733b76cdc30144b98c9b467a0f845"
allow_any_port = true
```
