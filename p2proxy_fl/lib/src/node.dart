import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:fluttertoast/fluttertoast.dart';
import 'package:p2proxy_fl/src/error_toast.dart';
import 'package:p2proxy_fl/src/notifications.dart';
import 'package:p2proxy_fl/src/rust/api/endpoint.dart';
import 'package:p2proxy_fl/src/rust/api/tokens.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:url_launcher/url_launcher.dart';

class NodeCard extends StatefulWidget {
  final UserDefinedNode node;
  final InitializedEndpoint endpoint;

  const NodeCard({super.key, required this.node, required this.endpoint});

  @override
  NodeCardState createState() {
    return NodeCardState();
  }
}

class NodeCardState extends State<NodeCard> {
  int? _port = 8080;
  Future<int?>? _pingFuture;
  StreamSubscription<String>? _sub;
  bool _conReady = false;
  final FToast _fToast = FToast();
  final FocusNode _portFocusNode = FocusNode();
  final FocusNode _namedPortFocusNode = FocusNode();
  final TextEditingController _portInputController = TextEditingController(
    text: "8080",
  );
  final TextEditingController _namedPortInputController =
      TextEditingController();

  @override
  void initState() {
    super.initState();
    _fToast.init(context);
  }

  @override
  void dispose() async {
    super.dispose();
    try {
      await _removeSub();
    } finally {
      var futures = [tearDownProxyNotification()];
      if (_sub != null) {
        futures.add(_sub!.cancel());
      }
      await Future.wait(futures);
    }
  }

  @override
  Widget build(BuildContext context) {
    /*WidgetsBinding.instance.addPostFrameCallback((d) {
      errorToast(_fToast, "test!");
    });*/
    VoidCallback? proxyOnPressed;
    if (_port != null) {
      proxyOnPressed = () async {
        try {
          PermissionStatus status =
              await Permission.ignoreBatteryOptimizations.status;
          if (!status.isGranted) {
            PermissionStatus askedStatus =
                await Permission.ignoreBatteryOptimizations.request();
            if (!askedStatus.isGranted) {
              errorToast(
                _fToast,
                "Denied ignore battery optimization permissions, app will not work",
              );
              return;
            }
          }
          _portFocusNode.unfocus();
          _namedPortFocusNode.unfocus();
          String? namedPort;
          if (_namedPortInputController.text.isNotEmpty) {
            namedPort = _namedPortInputController.text;
          }
          await _removeSub();
          _sub = widget.endpoint
              .serveRemoteTcp(
                port: _port!,
                address: widget.node,
                namedPort: namedPort,
              )
              .listen(
                (d) {
                  if (d == "s listening") {
                    setState(() {
                      _conReady = true;
                    });
                  } else if (d.startsWith("e")) {
                    errorToast(_fToast, d);
                  } else {
                    errorToast(_fToast, d);
                  }
                },
                onDone: () {
                  _tearDownSub();
                },
                onError: (e) {
                  _tearDownSub(err: e);
                },
              );
          requestNotificationsPermissionsIfPossible().then((isAllowed) {
            if (isAllowed) {
              return runProxyNotification(_port!);
            }
          });
        } on Exception catch (e) {
          errorToast(_fToast, e.toString());
        }
      };
    }
    VoidCallback? launchOnPressed;
    VoidCallback? stopOnPressed;
    if (_conReady) {
      launchOnPressed = () async {
        await launchUrl(Uri.parse("http://localhost:$_port"));
      };
      stopOnPressed = () async {
        if (_sub != null) {
          widget.endpoint.cancelStream();
          await _sub!.cancel();
          tearDownProxyNotification();
          _sub = null;
        }
        setState(() {
          _conReady = false;
        });
      };
    }
    List<Widget> children = [
      Padding(
        padding: EdgeInsets.only(top: 10, bottom: 15),
        child: Text("Connect to: ${widget.node.displayLabel()}"),
      ),
      Flex(
        direction: Axis.horizontal,
        mainAxisAlignment: MainAxisAlignment.spaceAround,
        children: [
          Flexible(
            child: ElevatedButton(
              onPressed: () {
                setState(() {
                  _pingFuture = widget.endpoint
                      .execPing(address: widget.node)
                      .onError((e, stack) {
                        errorToast(_fToast, "Ping failed: $e");
                        throw e!;
                      });
                });
              },
              child: const Text("Ping"),
            ),
          ),
          Flexible(child: pingProgress()),
        ],
      ),
      Padding(
        padding: EdgeInsets.only(top: 10),
        child: Flex(
          direction: Axis.horizontal,
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Flexible(
              flex: 2,
              child: Padding(
                padding: EdgeInsets.only(left: 20),
                child: Container(
                  constraints: BoxConstraints(maxWidth: 80),
                  child: TextField(
                    onEditingComplete: () {
                      _portFocusNode.unfocus();
                    },
                    onTapOutside: (e) {
                      _portFocusNode.unfocus();
                    },
                    onSubmitted: (s) {
                      _portFocusNode.unfocus();
                    },
                    controller: _portInputController,
                    focusNode: _portFocusNode,
                    decoration: InputDecoration(
                      label: const Text("local port"),
                      hintText: "localhost proxy port",
                    ),
                    inputFormatters: [
                      // 65536
                      LengthLimitingTextInputFormatter(5),
                    ],
                    keyboardType: TextInputType.numberWithOptions(
                      signed: true,
                      decimal: false,
                    ),
                    onChanged: (s) async {
                      try {
                        int port = int.parse(s, radix: 10);
                        setState(() {
                          _port = port;
                        });
                      } on Exception catch (e) {
                        errorToast(_fToast, "Bad port: $e");
                      }
                    },
                  ),
                ),
              ),
            ),
            Spacer(),
            Flexible(
              flex: 2,
              child: Container(
                constraints: BoxConstraints(maxWidth: 80),
                child: TextField(
                  onEditingComplete: () {
                    _namedPortFocusNode.unfocus();
                  },
                  onTapOutside: (e) {
                    _namedPortFocusNode.unfocus();
                  },
                  onSubmitted: (s) {
                    _namedPortFocusNode.unfocus();
                  },
                  controller: _namedPortInputController,
                  focusNode: _namedPortFocusNode,
                  decoration: InputDecoration(
                    label: const Text("Named port"),
                    hintText: "port name",
                  ),
                  inputFormatters: [LengthLimitingTextInputFormatter(16)],
                  keyboardType: TextInputType.text,
                ),
              ),
            ),
            Spacer(),
            Flexible(
              flex: 2,
              child: ElevatedButton(
                onPressed: proxyOnPressed,
                child: const Text("Proxy"),
              ),
            ),
          ],
        ),
      ),
      Padding(
        padding: EdgeInsets.only(top: 10, bottom: 10),
        child: Flex(
          direction: Axis.horizontal,
          mainAxisAlignment: MainAxisAlignment.spaceAround,
          children: [
            ElevatedButton(
              onPressed: launchOnPressed,
              child: const Text("Open"),
            ),
            ElevatedButton(onPressed: stopOnPressed, child: const Text("Stop")),
          ],
        ),
      ),
    ];
    return Card(margin: EdgeInsets.all(20), child: Column(children: children));
  }

  FutureBuilder<int?> pingProgress() {
    return FutureBuilder(
      future: _pingFuture,
      builder: (context, snapshot) {
        switch (snapshot.connectionState) {
          case ConnectionState.none:
            return const Text("Ping rtt: -");
          case ConnectionState.waiting:
            return const CircularProgressIndicator();
          case ConnectionState.active:
            return const CircularProgressIndicator();
          case ConnectionState.done:
            if (snapshot.hasError) {
              return const Text("Ping rtt: -");
            } else {
              return Text("Ping rtt: ${snapshot.requireData}ms");
            }
        }
      },
    );
  }

  Future<void> _removeSub() async {
    if (_sub != null) {
      widget.endpoint.cancelStream();
      await _sub!.cancel();
      tearDownProxyNotification();
      _sub = null;
    }
  }

  Future<void> _tearDownSub({String? err}) async {
    await _removeSub();
    if (err == null) {
      setState(() {
        _conReady = false;
      });
    } else {
      errorToast(_fToast, err);
      setState(() {
        _conReady = false;
      });
    }
  }
}
