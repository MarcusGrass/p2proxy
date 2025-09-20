import 'package:flutter/material.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:fluttertoast/fluttertoast.dart';
import 'package:p2proxy_fl/src/colorscheme.dart';
import 'package:p2proxy_fl/src/notifications.dart';
import 'package:p2proxy_fl/src/rust/frb_generated.dart';
import 'package:p2proxy_fl/src/storage.dart';
import 'package:p2proxy_fl/src/view.dart';

Future<void> main() async {
  await RustLib.init();
  final FlutterSecureStorage fss = FlutterSecureStorage();
  final Storage storage = Storage(inner: fss);
  WidgetsFlutterBinding.ensureInitialized();
  await initializeNotifications();
  runApp(MyApp(storage: storage));
}

class MyApp extends StatelessWidget {
  final Storage storage;

  const MyApp({super.key, required this.storage});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      builder: FToastBuilder(),
      title: 'P2Proxy',
      theme: colorScheme(),
      home: Scaffold(
        appBar: AppBar(
          title: const Text('p2proxy'),
          backgroundColor: Theme.of(context).colorScheme.secondary,
        ),
        body: MainView(storage: storage),
      ),
    );
  }
}
