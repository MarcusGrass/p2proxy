import 'package:flutter/material.dart';
import 'package:fluttertoast/fluttertoast.dart';
import 'package:p2proxy_fl/src/colorscheme.dart';
import 'package:p2proxy_fl/src/copy_on_click.dart';
import 'package:p2proxy_fl/src/error_toast.dart';
import 'package:p2proxy_fl/src/notifications.dart';
import 'package:p2proxy_fl/src/rust/api/endpoint.dart';
import 'package:p2proxy_fl/src/rust/api/tokens.dart';

class EndpointActivationWidget extends StatefulWidget {
  final UserDefinedKey userDefinedKey;
  final InitializedEndpoint? ep;
  final Function(InitializedEndpoint) onConnect;
  final VoidCallback onEndpointCancel;

  const EndpointActivationWidget({
    super.key,
    required this.userDefinedKey,
    this.ep,
    required this.onConnect,
    required this.onEndpointCancel,
  });

  @override
  EndpointActivationWidgetState createState() {
    return EndpointActivationWidgetState();
  }
}

class EndpointActivationWidgetState extends State<EndpointActivationWidget> {
  Future<void>? _endpointFuture;
  final FToast _fToast = FToast();

  @override
  void initState() {
    super.initState();
    _fToast.init(context);
  }

  @override
  Widget build(BuildContext context) {
    VoidCallback? onInitializePressed;
    VoidCallback? onClosePressed;
    if (widget.ep == null) {
      onInitializePressed = initializeFutureState();
    } else {
      onClosePressed = () {
        widget.onEndpointCancel();
        setState(() {
          _endpointFuture = null;
        });
      };
    }
    List<Widget> connectionRowChildren = [
      Flex(
        direction: Axis.horizontal,
        mainAxisAlignment: MainAxisAlignment.start,
        children: [
          ElevatedButton(
            onPressed: onInitializePressed,
            child: const Text("Connect"),
          ),
          Padding(padding: EdgeInsets.only(left: 10)),
          ElevatedButton(onPressed: onClosePressed, child: const Text("Close")),
        ],
      ),
      endpointLoad(),
    ];
    return Card(
      margin: EdgeInsets.all(20),
      child: Column(
        children: [
          Padding(
            padding: EdgeInsets.only(top: 10, left: 15, right: 15),
            child: InkWell(
              onTap: copyToClipboard(
                _fToast,
                widget.userDefinedKey.publicKeyHex(),
              ),
              child: Text(
                widget.userDefinedKey.displayLabel(),
                style: defaultText,
              ),
            ),
          ),
          Padding(
            padding: EdgeInsets.only(top: 10, bottom: 10, left: 20, right: 20),
            child: Flex(
              direction: Axis.horizontal,
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: connectionRowChildren,
            ),
          ),
        ],
      ),
    );
  }

  VoidCallback? initializeFutureState() {
    VoidCallback? onInitializePressed;
    if (_endpointFuture != null) {
      return onInitializePressed;
    }
    return () {
      // This should be kept void so that onConnect is only invoked once
      // and passed out of this widget
      Future<void> epFut = InitializedEndpoint.create(
        key: widget.userDefinedKey,
      ).then((ep) async {
        widget.onConnect(ep);
        bool canShowPerm = await requestNotificationsPermissionsIfPossible();
        if (canShowPerm) {
          runEndpointNotification();
        }
      });
      setState(() {
        _endpointFuture = epFut;
      });
    };
  }

  FutureBuilder endpointLoad() {
    return FutureBuilder(
      future: _endpointFuture,
      builder: (context, snapshot) {
        switch (snapshot.connectionState) {
          case ConnectionState.none:
            return const Icon(Icons.wifi, color: catRed);
          case ConnectionState.waiting:
            return CircularProgressIndicator();
          case ConnectionState.active:
            return CircularProgressIndicator();
          case ConnectionState.done:
            if (snapshot.hasError) {
              errorToast(_fToast, "Error ${snapshot.error}");
              return const Icon(Icons.wifi_off, color: catRed);
            }
            return const Icon(Icons.wifi, color: catGreen);
        }
      },
    );
  }
}
