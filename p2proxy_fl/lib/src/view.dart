import 'package:flutter/material.dart';
import 'package:fluttertoast/fluttertoast.dart';
import 'package:p2proxy_fl/src/activate_endpoint.dart';
import 'package:p2proxy_fl/src/error_toast.dart';
import 'package:p2proxy_fl/src/manage_keys.dart';
import 'package:p2proxy_fl/src/manage_nodes.dart';
import 'package:p2proxy_fl/src/node.dart';
import 'package:p2proxy_fl/src/notifications.dart';
import 'package:p2proxy_fl/src/rust/api/endpoint.dart';
import 'package:p2proxy_fl/src/rust/api/tokens.dart';
import 'package:p2proxy_fl/src/storage.dart';

import 'colorscheme.dart';

class MainView extends StatefulWidget {
  final Storage storage;

  const MainView({super.key, required this.storage});

  @override
  MainViewState createState() {
    return MainViewState();
  }
}

class MainViewState extends State<MainView> {
  UserDefinedKey? _selectedKey;
  InitializedEndpoint? _endpoint;
  UserDefinedNode? _selectedNode;
  Future<NodesAndKeys>? _fetchNodesAndKeys;
  final FToast _fToast = FToast();

  @override
  void initState() {
    super.initState();
    setState(() {
      _fetchNodesAndKeys = widget.storage.nodesAndKeys();
    });
    _fToast.init(context);
  }

  @override
  void dispose() {
    if (_endpoint != null) {
      _endpoint!.destroy();
      tearDownRunningNotifications();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder(
      future: _fetchNodesAndKeys,
      builder: (context, snapshot) {
        List<UserDefinedNode> nodes = [];
        List<UserDefinedKey> keys = [];
        switch (snapshot.connectionState) {
          case ConnectionState.none:
            break;
          case ConnectionState.waiting:
          case ConnectionState.active:
            return CircularProgressIndicator();
          case ConnectionState.done:
            if (snapshot.hasError) {
              errorToast(
                _fToast,
                "Failed to read stored data ${snapshot.error}",
              );
            } else {
              var data = snapshot.requireData;
              nodes = data.nodes;
              keys = data.keys;
            }
        }
        List<Widget> children = [
          Padding(padding: const EdgeInsets.only(top: 30)),
          createKeyChoice(keys),
        ];
        if (keys.length == 1) {
          _selectedKey = keys[0];
        }
        if (_selectedKey == null) {
          return ListView(children: children);
        }
        children.add(
          EndpointActivationWidget(
            userDefinedKey: _selectedKey!,
            ep: _endpoint,
            onConnect: (ep) {
              setState(() {
                _endpoint = ep;
              });
            },
            onEndpointCancel: () async {
              if (_endpoint != null) {
                // Don't await this, will cause UI lag
                _endpoint!.destroy();
                tearDownRunningNotifications();
                setState(() {
                  _endpoint = null;
                });
              }
            },
          ),
        );
        if (_endpoint == null) {
          return ListView(children: children);
        }

        if (nodes.length == 1) {
          _selectedNode = nodes[0];
        }
        if (_selectedNode == null) {
          children.addAll([
            Padding(padding: const EdgeInsets.only(top: 20)),
            createNodeChoice(nodes),
          ]);
          return ListView(children: children);
        }
        children.addAll([
          Padding(padding: const EdgeInsets.only(top: 20)),
          createNodeChoice(nodes),
          Padding(
            padding: EdgeInsets.only(top: 30),
            child: NodeCard(endpoint: _endpoint!, node: _selectedNode!),
          ),
        ]);
        return ListView(children: children);
      },
    );
  }

  Widget createKeyChoice(List<UserDefinedKey> keys) {
    return Padding(
      padding: EdgeInsets.only(left: 20),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.start,
        children: [
          keyDropdown(keys),
          IconButton(
            onPressed: () {
              bool keysAreDirty = false;
              Navigator.of(context)
                  .push(
                    MaterialPageRoute(
                      builder:
                          (context) => CreateKeyView(
                            storage: widget.storage,
                            keysDirty: () {
                              keysAreDirty = true;
                            },
                          ),
                    ),
                  )
                  .then((v) {
                    if (keysAreDirty) {
                      setState(() {
                        _fetchNodesAndKeys = widget.storage.nodesAndKeys();
                      });
                    }
                  });
            },
            icon: const Icon(Icons.add),
          ),
        ],
      ),
    );
  }

  Widget keyDropdown(List<UserDefinedKey> keys) {
    return dropDownSelection(
      "Secret key",
      keys,
      (k) {
        late String label = k.displayLabel();
        return DropDownInfo(label: label, value: k);
      },
      (k) {
        setState(() {
          _selectedKey = k;
        });
      },
    );
  }

  Widget createNodeChoice(List<UserDefinedNode> nodes) {
    return Padding(
      padding: EdgeInsets.only(left: 20),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.start,
        children: [
          nodeDropdown(nodes),
          IconButton(
            onPressed: () {
              bool nodesAreDirty = false;

              Navigator.of(context)
                  .push(
                    MaterialPageRoute(
                      builder:
                          (context) => ManageNodeView(
                            storage: widget.storage,
                            nodesDirty: () {
                              nodesAreDirty = true;
                            },
                          ),
                    ),
                  )
                  .then((v) {
                    if (nodesAreDirty) {
                      setState(() {
                        _fetchNodesAndKeys = widget.storage.nodesAndKeys();
                      });
                    }
                  });
            },
            icon: Icon(Icons.add),
          ),
        ],
      ),
    );
  }

  Widget nodeDropdown(List<UserDefinedNode> nodes) {
    return dropDownSelection(
      "Peer node id",
      nodes,
      (n) {
        String label = n.displayLabel();
        return DropDownInfo(label: label, value: n);
      },
      (n) {
        setState(() {
          _selectedNode = n;
        });
      },
    );
  }
}

class DropDownInfo<T> {
  final String label;
  final T value;

  DropDownInfo({required this.label, required this.value});
}

Widget dropDownSelection<T>(
  String hint,
  List<T> n,
  DropDownInfo Function(T) convert,
  ValueChanged<T> onSelect,
) {
  List<DropdownMenuEntry> entries = [];
  for (T ent in n) {
    DropDownInfo ddi = convert(ent);
    entries.add(
      DropdownMenuEntry(
        value: ddi.value,
        label: ddi.label,
        style: ButtonStyle(
          minimumSize: WidgetStatePropertyAll<Size>(Size(120, 20)),
        ),
      ),
    );
  }
  double? width;
  bool enabled = true;
  T? initialSelection;
  if (entries.isEmpty) {
    enabled = false;
    width = 180;
  } else if (entries.length == 1) {
    initialSelection = entries[0].value;
  }
  return DropdownMenu(
    enabled: enabled,
    width: width,
    hintText: hint,
    textStyle: defaultText,
    initialSelection: initialSelection,
    dropdownMenuEntries: entries,
    onSelected: (l) {
      if (l != null) {
        onSelect(l);
      }
    },
  );
}
