# P2proxy desktop

A cross-platform desktop application for launching `p2proxyd` services on localhost.

## Running

Either build from source `cargo r -r -p p2proxy-desktop`, or if you have a prebuilt binary,
just run that.

## Usage

There are only a few steps to using this application:

![image](../assets/images/demo-proxy-desktop.png)

1. Pick a secret key: From file, from hex, or by generating one.
2. Enter a peer node id
3. Ping it if you'd like
4. Pick a local port to serve on (8080 by default)
5. Use a named port, or press "proxy" to start serving
6. visit `http://localhost:<port>` or press "open".
