package io.github.zacharyk812.tillstone

import android.os.Bundle
import android.view.WindowManager
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Registers stay awake while the app is open.
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "tillstone/keepalive")
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "start" -> {
                        KeepAliveService.start(this)
                        result.success(null)
                    }
                    "status" -> result.success(KeepAliveService.status(this))
                    else -> result.notImplemented()
                }
            }
    }
}
