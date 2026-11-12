import 'dart:io';

import 'package:flutter/foundation.dart';

import 'rust/api/register.dart';
import 'vault.dart';

/// The app's state: the open register, who's signed in, the selling area
/// and the current order. Screens listen to it and call its methods; every
/// rule lives in Rust.
class Session extends ChangeNotifier {
  Session._(this.till, this.keyStorage, this.vault, this.state);

  final Till till;
  final KeyStorage keyStorage;
  final KeyVault vault;
  DeviceState state;

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
    return Session._(till, storage, vault, await vault.readState());
  }

  /// Creates the demo business on this device.
  static Future<Session> createDemo(KeyVault vault) => _create(
    vault,
    demo: true,
    (db, keys) => Till.createDemo(dbPath: db, dbKey: keys.database, deviceSecret: keys.device),
  );

  /// Sets up a real business from the wizard's answers.
  static Future<Session> create(KeyVault vault, SetupInput setup) => _create(
    vault,
    demo: false,
    (db, keys) => Till.create(dbPath: db, dbKey: keys.database, deviceSecret: keys.device, setup: setup),
  );

  static Future<Session> _create(
    KeyVault vault,
    Future<Till> Function(String db, DeviceKeys keys) make, {
    required bool demo,
  }) async {
    final (keys, storage) = await vault.load();
    final db = await _dbFile(vault);
    final till = await make(db.path, keys);
    final kit = till.takeOwnerKit()!;
    // Save the sealed owner key before anything else, so the recovery code
    // can always be shown again with the passphrase.
    final state = DeviceState(demo: demo, recoveryConfirmed: demo, sealedOwnerKey: kit.sealedOwnerKey);
    await vault.writeState(state);
    return Session._(till, storage, vault, state)..recoveryCode = kit.recoveryCode;
  }

  /// The recovery code from setup, held only until the owner confirms it.
  String? recoveryCode;

  /// Shows the recovery code again from the sealed key and passphrase.
  Future<String> revealRecoveryCode(String passphrase) =>
      till.recoveryCode(sealedOwnerKey: state.sealedOwnerKey!, passphrase: passphrase);

  /// The owner saved the recovery code; setup is complete.
  Future<void> confirmRecovery() async {
    state = state.copyWith(recoveryConfirmed: true);
    await vault.writeState(state);
    recoveryCode = null;
    notifyListeners();
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
