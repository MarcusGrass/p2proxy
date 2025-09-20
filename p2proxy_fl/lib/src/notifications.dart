import 'dart:io';

import 'package:flutter_local_notifications/flutter_local_notifications.dart';

Future<void> initializeNotifications() {
  const InitializationSettings settings = InitializationSettings(
    android: AndroidInitializationSettings("p2proxy48_notification"),
  );
  return FlutterLocalNotificationsPlugin().initialize(
    settings,
    onDidReceiveNotificationResponse: onNotificationResponse,
  );
}

Future<void> onNotificationResponse(NotificationResponse response) async {
  // Could remove this, actions taken on notifications run on a
  // separate isolate, which becomes monumentally annoying to deal with.
  // So, will not implement a 'stop button' on the notification.
}

const int endpointRunningId = 774;
const String endpointRunningAndroidChannelId = "p2proxy-endpoint-running";
const String endpointRunningAndroidChannelName = "p2proxy endpoint running";
const String endpointRunningAdroidChannelDesc =
    "p2proxy is currently running an endpoint, this draws power in the background. "
    "Therefore, you should close the app if this notification is showing and "
    "you're not currently using it";

Future<void> runEndpointNotification() async {
  return FlutterLocalNotificationsPlugin().show(
    endpointRunningId,
    "P2proxy endpoint running",
    "Endpoint running in the background ready to create connections",
    getDetails(
      endpointRunningAndroidChannelId,
      endpointRunningAndroidChannelName,
    ),
  );
}

Future<void> tearDownRunningNotifications() async {
  Future.wait([
    FlutterLocalNotificationsPlugin().cancel(endpointRunningId),
    FlutterLocalNotificationsPlugin().cancel(proxyRunningId),
  ]);
}

const int proxyRunningId = 775;
const String proxyRunningAndroidChannelId = "p2proxy-proxy-running";
const String proxyRunningAndroidChannelName = "p2proxy proxy running";
const String proxyRunningAndroidChannelDesc =
    "p2proxy is currently proxying data, this draws power in the background. "
    "Therefore, you should close the app if this notification is showing and "
    "you're not currently using it";

Future<bool> requestNotificationsPermissionsIfPossible() async {
  FlutterLocalNotificationsPlugin plugin = FlutterLocalNotificationsPlugin();
  if (Platform.isAndroid) {
    final androidPlugin =
        plugin
            .resolvePlatformSpecificImplementation<
              AndroidFlutterLocalNotificationsPlugin
            >()!;
    bool? isEnabled = await androidPlugin.areNotificationsEnabled();
    if (isEnabled == null || !isEnabled) {
      bool? response = await androidPlugin.requestNotificationsPermission();
      if (response != null) {
        return response;
      } else {
        return false;
      }
    }
    return true;
  }
  return false;
}

Future<void> runProxyNotification(int port) async {
  return FlutterLocalNotificationsPlugin().show(
    proxyRunningId,
    "P2proxy running",
    "Proxying to http://localhost:$port",
    getDetails(proxyRunningAndroidChannelId, proxyRunningAndroidChannelName),
  );
}

NotificationDetails getDetails(String channelId, String channelName) {
  return NotificationDetails(
    android: AndroidNotificationDetails(
      channelId,
      channelName,
      channelDescription: proxyRunningAndroidChannelDesc,
      priority: Priority.low,
      ongoing: true,
      autoCancel: false,
      playSound: false,
      enableVibration: false,
      enableLights: false,
    ),
  );
}

Future<void> tearDownProxyNotification() async {
  return FlutterLocalNotificationsPlugin().cancel(proxyRunningId);
}
