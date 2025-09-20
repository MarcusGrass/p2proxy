import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:fluttertoast/fluttertoast.dart';
import 'package:p2proxy_fl/src/error_toast.dart';

GestureTapCallback copyToClipboard(FToast fToast, String text) {
  return () {
    Clipboard.setData(ClipboardData(text: text)).then((o) {
      infoToast(fToast, "Copied $text to clipboard");
    });
  };
}
