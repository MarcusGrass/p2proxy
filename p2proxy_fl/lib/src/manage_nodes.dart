import 'package:flutter/material.dart';
import 'package:fluttertoast/fluttertoast.dart';
import 'package:p2proxy_fl/src/colorscheme.dart';
import 'package:p2proxy_fl/src/error_toast.dart';
import 'package:p2proxy_fl/src/rust/api/tokens.dart';
import 'package:p2proxy_fl/src/storage.dart';

import 'copy_on_click.dart';

class ManageNodeView extends StatefulWidget {
  const ManageNodeView({
    super.key,
    required this.storage,
    required this.nodesDirty,
  });

  final Storage storage;
  final VoidCallback nodesDirty;

  @override
  ManageNodeViewState createState() {
    return ManageNodeViewState();
  }
}

class ManageNodeViewState extends State<ManageNodeView> {
  Future<List<UserDefinedNode>>? _loadNodesFuture;
  final List<bool> _openExpansions = [];
  final GlobalKey<FormState> _formKey = GlobalKey<FormState>();
  final TextEditingController _nodeIdController = TextEditingController();
  final TextEditingController _nameController = TextEditingController();
  final FToast _fToast = FToast();

  @override
  void initState() {
    super.initState();
    _loadNodesFuture = widget.storage.nodes();
    _fToast.init(context);
  }

  @override
  Widget build(BuildContext context) {
    List<Widget> columnChildren = [
      Padding(
        padding: EdgeInsets.only(top: 15, left: 10, right: 10),
        child: Card(
          child: Padding(
            padding: EdgeInsets.only(top: 10, left: 15, right: 15),
            child: newNodeForm(context),
          ),
        ),
      ),
      fetchedNodesWidget(),
    ];

    return Scaffold(
      appBar: AppBar(title: const Text('Nodes'), backgroundColor: appBarColor),
      body: ListView(shrinkWrap: true, children: columnChildren),
    );
  }

  Form newNodeForm(BuildContext context) {
    return Form(
      key: _formKey,
      child: Column(
        children: [
          const Text("Add a new node"),
          TextFormField(
            decoration: const InputDecoration(
              label: Text("Node id"),
              hintText: "Node id hexadecimal",
            ),
            controller: _nodeIdController,
            validator: (String? value) {
              if (value == null || value.length != 64) {
                return "Node id must be a valid 64 byte hexadecimal";
              }
              return null;
            },
          ),
          TextFormField(
            decoration: const InputDecoration(
              label: Text("Key alias"),
              hintText: "Any name to remember the key by",
            ),
            controller: _nameController,
          ),
          Padding(
            padding: const EdgeInsets.only(top: 24, bottom: 12),
            child: ElevatedButton(
              onPressed: () async {
                if (!_formKey.currentState!.validate()) {
                  return;
                }
                String? name;
                if (_nameController.text.isNotEmpty) {
                  name = _nameController.text;
                }
                try {
                  UserDefinedNode node = UserDefinedNode.tryNew(
                    nodeId: _nodeIdController.text,
                    name: name,
                  );
                  setState(() {
                    _loadNodesFuture = widget.storage.storeNode(node);
                  });
                  widget.nodesDirty();
                } on Exception catch (e) {
                  errorToast(_fToast, "Error creating node $e");
                }
              },
              child: const Text("Create"),
            ),
          ),
        ],
      ),
    );
  }

  FutureBuilder<List<UserDefinedNode>> fetchedNodesWidget() {
    return FutureBuilder(
      future: _loadNodesFuture,
      builder: (context, snapshot) {
        switch (snapshot.connectionState) {
          case ConnectionState.none:
          case ConnectionState.waiting:
          case ConnectionState.active:
            return CircularProgressIndicator();
          case ConnectionState.done:
            if (snapshot.hasError) {
              errorToast(_fToast, "Failed to fetch nodes: ${snapshot.error}");
              return Column(children: []);
            }
            List<UserDefinedNode> data = snapshot.requireData;
            _openExpansions.clear();
            for (var k in data) {
              _openExpansions.add(false);
            }
            if (data.isEmpty) {
              return Column(children: []);
            }

            return Padding(
              padding: EdgeInsets.only(top: 5, left: 10, right: 10),
              child: Card(
                child: Padding(
                  padding: EdgeInsets.only(top: 15),
                  child: loadedNodesWidget(data),
                ),
              ),
            );
        }
      },
    );
  }

  Widget loadedNodesWidget(List<UserDefinedNode> nodes) {
    List<Widget> children = [const Text("Known nodes")];
    for (final (int i, UserDefinedNode node) in nodes.indexed) {
      String label = node.displayLabel();
      children.add(
        Padding(
          padding: EdgeInsets.all(10),
          child: ExpansionTile(
            title: Text(label),
            initiallyExpanded: _openExpansions[i],
            onExpansionChanged: (open) {
              _openExpansions[i] = open;
            },
            children: [
              InkWell(
                onTap: copyToClipboard(_fToast, node.address()),
                child: Text("Public key: ${node.address()}"),
              ),
              Flex(
                direction: Axis.horizontal,
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  ElevatedButton(
                    onPressed: () {
                      setState(() {
                        _loadNodesFuture = widget.storage.deleteNode(node);
                        widget.nodesDirty();
                      });
                    },
                    child: const Text("Delete"),
                  ),
                ],
              ),
            ],
          ),
        ),
      );
    }
    return Column(children: children);
  }
}
