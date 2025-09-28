import 'package:flutter/material.dart';
import 'package:fluttertoast/fluttertoast.dart';
import 'package:p2proxy_fl/src/colorscheme.dart';

void errorToast(FToast fToast, String error) {
  Widget toast = Container(
    padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
    constraints: BoxConstraints(minWidth: 120),
    decoration: BoxDecoration(
      color: catOnRed,
      borderRadius: BorderRadius.all(Radius.circular(10.0)),
    ),
    child: Center(
      widthFactor: 1,
      child: Text(error, style: TextStyle(color: catRed)),
    ),
  );
  fToast.removeQueuedCustomToasts();
  fToast.removeCustomToast();
  fToast.showToast(
    child: toast,
    gravity: ToastGravity.BOTTOM,
    isDismissible: true,
    toastDuration: Duration(seconds: 5),
  );
}

void infoToast(FToast fToast, String message) {
  Widget toast = Container(
    padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
    constraints: BoxConstraints(minWidth: 120),
    decoration: BoxDecoration(
      color: catOnRed,
      borderRadius: BorderRadius.all(Radius.circular(10.0)),
    ),
    child: Center(widthFactor: 1, child: Text(message, style: defaultText)),
  );
  fToast.showToast(
    child: toast,
    gravity: ToastGravity.BOTTOM,
    isDismissible: true,
    toastDuration: Duration(seconds: 5),
  );
}
