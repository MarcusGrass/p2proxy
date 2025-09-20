import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:p2proxy_fl/src/rust/api/tokens.dart';

bool storeNodeMutexLocked = false;
bool storeKeyMutexLocked = false;

const String nodeStorageKey = "nodes";
const String secretKeyStorageKey = "keys";

class Storage {
  final FlutterSecureStorage inner;

  Storage({required this.inner});

  Future<NodesAndKeys> nodesAndKeys() {
    return nodes().then((nodes) async {
      List<UserDefinedKey> keys = await this.keys();
      return NodesAndKeys(nodes: nodes, keys: keys);
    });
  }

  Future<List<UserDefinedNode>> nodes() async {
    String? value = await inner.read(key: nodeStorageKey);
    if (value == null) {
      return [];
    }
    return UserDefinedNode.deserializeMany(s: value);
  }

  Future<List<UserDefinedNode>> storeNode(UserDefinedNode node) async {
    while (storeNodeMutexLocked) {
      await Future.delayed(Duration(milliseconds: 10));
    }
    try {
      storeNodeMutexLocked = true;
      List<UserDefinedNode> nodes = await this.nodes();
      final (
        String serialized,
        List<UserDefinedNode> nodeRet,
      ) = await UserDefinedNode.addAndSerialize(many: nodes, tgt: node);
      await inner.write(key: nodeStorageKey, value: serialized);
      return nodeRet;
    } finally {
      storeNodeMutexLocked = false;
    }
  }

  Future<List<UserDefinedKey>> keys() async {
    String? value = await inner.read(key: secretKeyStorageKey);
    if (value == null) {
      return [];
    }
    return UserDefinedKey.deserializeMany(s: value);
  }

  Future<List<UserDefinedKey>> storeKeys(UserDefinedKey key) async {
    while (storeKeyMutexLocked) {
      await Future.delayed(Duration(milliseconds: 10));
    }
    try {
      storeKeyMutexLocked = true;
      List<UserDefinedKey> keys = await this.keys();
      final (
        String serialized,
        List<UserDefinedKey> keysRet,
      ) = await UserDefinedKey.addAndSerialize(many: keys, tgt: key);
      await inner.write(key: secretKeyStorageKey, value: serialized);
      return keysRet;
    } finally {
      storeKeyMutexLocked = false;
    }
  }

  Future<List<UserDefinedKey>> deleteKey(UserDefinedKey key) async {
    while (storeKeyMutexLocked) {
      await Future.delayed(Duration(milliseconds: 10));
    }
    try {
      storeKeyMutexLocked = true;
      List<UserDefinedKey> keys = await this.keys();
      final (
        String? serialized,
        List<UserDefinedKey> keysRet,
      ) = await UserDefinedKey.removeAndSerializeIfPresent(
        many: keys,
        tgt: key,
      );
      if (serialized == null) {
        return keysRet;
      }
      await inner.write(key: secretKeyStorageKey, value: serialized);
      return keysRet;
    } finally {
      storeKeyMutexLocked = false;
    }
  }

  Future<List<UserDefinedNode>> deleteNode(UserDefinedNode node) async {
    while (storeNodeMutexLocked) {
      await Future.delayed(Duration(milliseconds: 10));
    }
    try {
      storeNodeMutexLocked = true;
      List<UserDefinedNode> nodes = await this.nodes();
      final (
        String? serialized,
        List<UserDefinedNode> nodesRet,
      ) = await UserDefinedNode.removeAndSerializeIfPresent(
        many: nodes,
        tgt: node,
      );
      if (serialized == null) {
        return nodesRet;
      }
      await inner.write(key: nodeStorageKey, value: serialized);
      return nodesRet;
    } finally {
      storeNodeMutexLocked = false;
    }
  }
}

class NodesAndKeys {
  final List<UserDefinedNode> nodes;
  final List<UserDefinedKey> keys;

  NodesAndKeys({required this.nodes, required this.keys});
}
