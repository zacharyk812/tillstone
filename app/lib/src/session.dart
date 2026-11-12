import 'dart:io';

import 'package:flutter/foundation.dart';

import 'rust/api/register.dart';
import 'vault.dart';

/// The app's state: the open register, who's signed in, the selling area
/// and the current order. Screens listen to it and call its methods; every
/// rule lives in Rust.
class Session extends ChangeNotifier {
  Session._(this.till, this.keyStorage);

  final Till till;
  final KeyStorage keyStorage;

  StaffView? staff;
  CenterView? center;
  String? orderId;
  OrderView? order;
  String? error;
  bool busy = false;

  /// Opens this device's register, or returns null if none exists yet.
  static Future<Session?> tryOpen(KeyVault vault) async {
    final (keys, storage) = await vault.load();
    final db = await _dbFile(vault);
    if (!db.existsSync()) return null;
    final till = await Till.open(dbPath: db.path, dbKey: keys.database, deviceSecret: keys.device);
    return Session._(till, storage);
  }

  /// Creates the demo business on this device.
  static Future<Session> createDemo(KeyVault vault) async {
    final (keys, storage) = await vault.load();
    final db = await _dbFile(vault);
    final till = await Till.createDemo(
      dbPath: db.path,
      dbKey: keys.database,
      deviceSecret: keys.device,
      ownerSecret: keys.owner,
    );
    return Session._(till, storage);
  }

  static Future<File> _dbFile(KeyVault vault) async {
    final dir = await vault.appDirectory();
    dir.createSync(recursive: true);
    return File('${dir.path}/tillstone.db');
  }

  List<CenterView> get centers => till.revenueCenters();

  List<ItemView> get items => center == null ? const [] : till.items(centerId: center!.id);

  Future<T?> _run<T>(Future<T> Function() action) async {
    busy = true;
    error = null;
    notifyListeners();
    try {
      return await action();
    } on Object catch (e) {
      error = e.toString();
      return null;
    } finally {
      busy = false;
      _refreshOrder();
      notifyListeners();
    }
  }

  void _refreshOrder() {
    order = orderId == null ? null : till.order(orderId: orderId!);
  }

  Future<bool> signIn(String pin, {String? staffId}) async {
    final s = await _run(() => till.signIn(pin: pin, staffId: staffId));
    if (s == null) return false;
    staff = s;
    center ??= centers.isEmpty ? null : centers.first;
    notifyListeners();
    return true;
  }

  void signOut() {
    till.signOut();
    staff = null;
    orderId = null;
    order = null;
    notifyListeners();
  }

  void selectCenter(CenterView c) {
    center = c;
    orderId = null;
    order = null;
    notifyListeners();
  }

  Future<void> addItem(ItemView item, {String? variantId, List<String> optionIds = const []}) => _run(() async {
    orderId ??= await till.openOrder(centerId: center!.id, fulfillment: 'here');
    await till.addItem(orderId: orderId!, itemId: item.id, variantId: variantId, optionIds: optionIds, quantity: 1);
  });

  Future<void> setQuantity(LineView line, int quantity) =>
      _run(() => till.setQuantity(orderId: orderId!, lineId: line.id, quantity: quantity));

  Future<void> discount(String name, int basisPoints, {String? approverPin}) =>
      _run(() => till.discount(orderId: orderId!, name: name, basisPoints: basisPoints, approverPin: approverPin));

  Future<CashView?> payCash(int givenMinor) => _run(() => till.payCash(orderId: orderId!, givenMinor: givenMinor));

  /// Starts a fresh order after a completed sale.
  void newOrder() {
    orderId = null;
    order = null;
    notifyListeners();
  }
}
