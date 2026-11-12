import 'dart:async';

import 'package:tillstone_design_system/testing.dart';

/// Runs before every test file in this folder.
Future<void> testExecutable(FutureOr<void> Function() testMain) async {
  useTolerantGoldens();
  await testMain();
}
