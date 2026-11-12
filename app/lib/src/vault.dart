import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:path_provider/path_provider.dart';

/// The keys this device needs (PLAN §9).
class DeviceKeys {
  const DeviceKeys({required this.database, required this.device, required this.owner});

  /// Encrypts the local database (SQLCipher).
  final Uint8List database;

  /// This device's identity: signs its records and its connections.
  final Uint8List device;

  /// The owner authority. TODO(M4): keep it encrypted with the owner
  /// passphrase instead of next to the other keys.
  final Uint8List owner;
}

/// Where the keys ended up.
enum KeyStorage {
  /// The operating system's secure store (Keychain, Android Keystore,
  /// Windows Credential Manager, Linux Secret Service).
  system,

  /// A file only this user can read. Used only when no system store is
  /// available (e.g. Linux without a keyring); the app shows a warning.
  file,
}

/// Loads this device's keys, creating them on first launch.
class KeyVault {
  KeyVault({FlutterSecureStorage? storage, Directory? directory})
    : _storage =
          storage ??
          const FlutterSecureStorage(
            // The data-protection keychain needs a signed app with keychain
            // entitlements; the login keychain works for development builds.
            mOptions: MacOsOptions(usesDataProtectionKeychain: false),
          ),
      _directory = directory; // ignore: prefer_initializing_formals

  final FlutterSecureStorage _storage;
  final Directory? _directory;

  static const _names = ['database', 'device', 'owner'];
  static const _prefix = 'tillstone.key.';

  Future<Directory> appDirectory() async => _directory ?? await getApplicationSupportDirectory();

  Future<(DeviceKeys, KeyStorage)> load() async {
    try {
      return (await _loadFrom(_readSystem, _writeSystem), KeyStorage.system);
    } on Object {
      final dir = await appDirectory();
      final file = File('${dir.path}/keys.json');
      Future<String?> read(String name) async {
        if (!file.existsSync()) return null;
        return (jsonDecode(file.readAsStringSync()) as Map<String, dynamic>)[name] as String?;
      }

      Future<void> write(String name, String value) async {
        final map = file.existsSync()
            ? jsonDecode(file.readAsStringSync()) as Map<String, dynamic>
            : <String, dynamic>{};
        map[name] = value;
        file.parent.createSync(recursive: true);
        file.writeAsStringSync(jsonEncode(map));
        if (!Platform.isWindows) {
          await Process.run('chmod', ['600', file.path]);
        }
      }

      return (await _loadFrom(read, write), KeyStorage.file);
    }
  }

  Future<String?> _readSystem(String name) => _storage.read(key: '$_prefix$name');

  Future<void> _writeSystem(String name, String value) => _storage.write(key: '$_prefix$name', value: value);

  Future<DeviceKeys> _loadFrom(
    Future<String?> Function(String) read,
    Future<void> Function(String, String) write,
  ) async {
    final keys = <String, Uint8List>{};
    for (final name in _names) {
      var value = await read(name);
      if (value == null) {
        value = base64Encode(_random32());
        await write(name, value);
        // Read back, so a store that silently drops writes is caught now.
        if (await read(name) != value) throw StateError('key store did not keep the key');
      }
      keys[name] = base64Decode(value);
    }
    return DeviceKeys(database: keys['database']!, device: keys['device']!, owner: keys['owner']!);
  }

  static Uint8List _random32() {
    final rng = Random.secure();
    return Uint8List.fromList(List.generate(32, (_) => rng.nextInt(256)));
  }
}
