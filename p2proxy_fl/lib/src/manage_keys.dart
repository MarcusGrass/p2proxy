import 'package:flutter/material.dart';
import 'package:fluttertoast/fluttertoast.dart';
import 'package:p2proxy_fl/src/colorscheme.dart';
import 'package:p2proxy_fl/src/copy_on_click.dart';
import 'package:p2proxy_fl/src/error_toast.dart';
import 'package:p2proxy_fl/src/rust/api/tokens.dart';
import 'package:p2proxy_fl/src/storage.dart';

class CreateKeyView extends StatefulWidget {
  final Storage storage;
  final VoidCallback keysDirty;

  const CreateKeyView({
    super.key,
    required this.storage,
    required this.keysDirty,
  });

  @override
  CreateKeyViewState createState() {
    return CreateKeyViewState();
  }
}

class CreateKeyViewState extends State<CreateKeyView> {
  Future<List<UserDefinedKey>>? _loadKeysFuture;
  final List<bool> _openExpansions = [];
  final GlobalKey<FormState> _formKey = GlobalKey<FormState>();
  final TextEditingController _secretKeyHexController = TextEditingController();
  final TextEditingController _nameController = TextEditingController();
  final FToast _fToast = FToast();

  @override
  void initState() {
    _loadKeysFuture = widget.storage.keys();
    super.initState();
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
            child: newKeyForm(context),
          ),
        ),
      ),
      fetchedKeysWidget(),
    ];

    return Scaffold(
      appBar: AppBar(title: const Text('Keys'), backgroundColor: appBarColor),
      body: ListView(shrinkWrap: true, children: columnChildren),
    );
  }

  Form newKeyForm(BuildContext context) {
    return Form(
      key: _formKey,
      child: Column(
        children: [
          const Text("Create a new key"),
          Flex(
            direction: Axis.horizontal,
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Flexible(
                flex: 8,
                child: TextFormField(
                  decoration: const InputDecoration(
                    label: Text("Secret key"),
                    hintText: "Secret key hexadecimal",
                  ),
                  controller: _secretKeyHexController,
                  validator: (String? value) {
                    if (value == null || value.length != 64) {
                      return "Secret key must be a valid 64 byte hexadecimal";
                    }
                    return null;
                  },
                ),
              ),
              Flexible(
                child: IconButton(
                  onPressed: () {
                    String key = UserDefinedKey.generateKey();
                    _secretKeyHexController.text = key;
                  },
                  icon: Icon(Icons.refresh_outlined, color: catGreen),
                ),
              ),
            ],
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
                setState(() {
                  _loadKeysFuture = widget.storage.storeKeys(
                    UserDefinedKey.generateWithName(name: name),
                  );
                });
                widget.keysDirty();
              },
              child: const Text("Create"),
            ),
          ),
        ],
      ),
    );
  }

  FutureBuilder<List<UserDefinedKey>> fetchedKeysWidget() {
    return FutureBuilder(
      future: _loadKeysFuture,
      builder: (context, snapshot) {
        switch (snapshot.connectionState) {
          case ConnectionState.none:
          case ConnectionState.waiting:
          case ConnectionState.active:
            return CircularProgressIndicator();
          case ConnectionState.done:
            if (snapshot.hasError) {
              errorToast(_fToast, "Failed to fetch keys: ${snapshot.error}");
              return Column(children: []);
            }
            List<UserDefinedKey> data = snapshot.requireData;
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
                  child: loadedKeysWidget(data),
                ),
              ),
            );
        }
      },
    );
  }

  Widget loadedKeysWidget(List<UserDefinedKey> keys) {
    List<Widget> children = [const Text("Existing keys")];
    for (final (int i, UserDefinedKey key) in keys.indexed) {
      String label = key.displayLabel();
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
                onTap: copyToClipboard(_fToast, key.publicKeyHex()),
                child: Text("Public key: ${key.publicKeyHex()}"),
              ),
              Flex(
                direction: Axis.horizontal,
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  ElevatedButton(
                    onPressed: () {
                      setState(() {
                        _loadKeysFuture = widget.storage.deleteKey(key);
                        widget.keysDirty();
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
