import 'dart:io' show Platform;

import 'package:flutter/services.dart';

/// Keeps the app running in the background on Android (PLAN §13). Desktop
/// platforms don't suspend apps, so this does nothing there.
abstract final class BackgroundService {
  static const _channel = MethodChannel('tillstone/keepalive');

  static bool get supported => Platform.isAndroid;

  static Future<void> start() async {
    if (supported) await _channel.invokeMethod<void>('start');
  }

  /// Survival statistics from the background service, or null if unsupported.
  static Future<BackgroundServiceStatus?> status() async {
    if (!supported) return null;
    final m = await _channel.invokeMapMethod<String, int>('status');
    if (m == null) return null;
    return BackgroundServiceStatus(
      started: DateTime.fromMillisecondsSinceEpoch(m['startedMs'] ?? 0),
      lastHeartbeat: DateTime.fromMillisecondsSinceEpoch(m['lastHeartbeatMs'] ?? 0),
      heartbeats: m['heartbeats'] ?? 0,
      maxGap: Duration(milliseconds: m['maxGapMs'] ?? 0),
    );
  }
}

class BackgroundServiceStatus {
  const BackgroundServiceStatus({
    required this.started,
    required this.lastHeartbeat,
    required this.heartbeats,
    required this.maxGap,
  });

  final DateTime started;
  final DateTime lastHeartbeat;
  final int heartbeats;

  /// Largest gap between one-minute heartbeats. Much more than a minute means
  /// Android paused or killed the app at some point.
  final Duration maxGap;

  bool get passed => heartbeats > 0 && maxGap < const Duration(minutes: 3);

  @override
  String toString() {
    final running = lastHeartbeat.difference(started);
    return 'Background service: $heartbeats heartbeats over '
        '${running.inHours}h ${running.inMinutes % 60}m, '
        'largest gap ${maxGap.inSeconds}s';
  }
}
