package org.nativecod.app

import android.net.Uri
import android.os.Bundle
import android.provider.DocumentsContract
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.WindowInsets
import com.google.androidgamesdk.GameActivity
import kotlin.math.abs

class RuntimeActivity : GameActivity() {
    companion object { init { System.loadLibrary("android_runtime") } }
    private external fun nativeConfigure(root: String, files: String, diagnostic: Boolean, source: SafStore?)
    private external fun nativeRequestExit()
    private external fun nativePad(lx: Float, ly: Float, rx: Float, ry: Float, lt: Float, rt: Float, buttons: Int)
    private var keyButtons = 0
    private var hatButtons = 0
    private var lx = 0f
    private var ly = 0f
    private var rx = 0f
    private var ry = 0f
    private var lt = 0f
    private var rt = 0f
    override fun onCreate(savedInstanceState: Bundle?) {
        val tree = intent.getStringExtra("tree") ?: ""
        val root = primaryPath(tree)
        nativeConfigure(root, filesDir.absolutePath, intent.getBooleanExtra("diagnostic", false),
            if (tree.isEmpty()) null else SafStore(this, tree))
        super.onCreate(savedInstanceState)
        window.insetsController?.hide(WindowInsets.Type.systemBars())
    }
    override fun onDestroy() {
        super.onDestroy()
        android.os.Process.killProcess(android.os.Process.myPid())
    }
    @Deprecated("GameActivity back dispatch")
    override fun onBackPressed() { nativeRequestExit() }
    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        if (event.keyCode == KeyEvent.KEYCODE_BACK) {
            if (event.action == KeyEvent.ACTION_UP) nativeRequestExit()
            return true
        }
        val bit = buttonBit(event.keyCode)
        if (bit != 0 && event.repeatCount == 0) {
            keyButtons = if (event.action == KeyEvent.ACTION_DOWN) keyButtons or bit else keyButtons and bit.inv()
            publish()
        }
        return super.dispatchKeyEvent(event)
    }
    override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean {
        if (event.isFromSource(InputDevice.SOURCE_JOYSTICK) || event.isFromSource(InputDevice.SOURCE_GAMEPAD)) {
            lx = event.getAxisValue(MotionEvent.AXIS_X)
            ly = event.getAxisValue(MotionEvent.AXIS_Y)
            val z = event.getAxisValue(MotionEvent.AXIS_Z) to event.getAxisValue(MotionEvent.AXIS_RZ)
            val r = event.getAxisValue(MotionEvent.AXIS_RX) to event.getAxisValue(MotionEvent.AXIS_RY)
            val right = if (abs(z.first) + abs(z.second) >= abs(r.first) + abs(r.second)) z else r
            rx = right.first
            ry = right.second
            lt = maxOf(event.getAxisValue(MotionEvent.AXIS_LTRIGGER), event.getAxisValue(MotionEvent.AXIS_BRAKE))
            rt = maxOf(event.getAxisValue(MotionEvent.AXIS_RTRIGGER), event.getAxisValue(MotionEvent.AXIS_GAS))
            val hatX = event.getAxisValue(MotionEvent.AXIS_HAT_X)
            val hatY = event.getAxisValue(MotionEvent.AXIS_HAT_Y)
            hatButtons = 0
            if (hatY < -0.5f) hatButtons = hatButtons or (1 shl 10)
            if (hatY > 0.5f) hatButtons = hatButtons or (1 shl 11)
            if (hatX < -0.5f) hatButtons = hatButtons or (1 shl 12)
            if (hatX > 0.5f) hatButtons = hatButtons or (1 shl 13)
            publish()
        }
        return super.dispatchGenericMotionEvent(event)
    }
    private fun publish() { nativePad(lx, ly, rx, ry, lt, rt, keyButtons or hatButtons) }
}

private fun buttonBit(keyCode: Int) = when (keyCode) {
    KeyEvent.KEYCODE_BUTTON_A, KeyEvent.KEYCODE_DPAD_CENTER -> 1
    KeyEvent.KEYCODE_BUTTON_B -> 1 shl 1
    KeyEvent.KEYCODE_BUTTON_X -> 1 shl 2
    KeyEvent.KEYCODE_BUTTON_Y -> 1 shl 3
    KeyEvent.KEYCODE_BUTTON_L1 -> 1 shl 4
    KeyEvent.KEYCODE_BUTTON_R1 -> 1 shl 5
    KeyEvent.KEYCODE_BUTTON_L2 -> 1 shl 6
    KeyEvent.KEYCODE_BUTTON_R2 -> 1 shl 7
    KeyEvent.KEYCODE_BUTTON_SELECT -> 1 shl 8
    KeyEvent.KEYCODE_BUTTON_START -> 1 shl 9
    KeyEvent.KEYCODE_DPAD_UP -> 1 shl 10
    KeyEvent.KEYCODE_DPAD_DOWN -> 1 shl 11
    KeyEvent.KEYCODE_DPAD_LEFT -> 1 shl 12
    KeyEvent.KEYCODE_DPAD_RIGHT -> 1 shl 13
    KeyEvent.KEYCODE_BUTTON_THUMBL -> 1 shl 14
    KeyEvent.KEYCODE_BUTTON_THUMBR -> 1 shl 15
    else -> 0
}

private fun primaryPath(tree: String): String {
    if (tree.isEmpty()) return ""
    val id = try { DocumentsContract.getTreeDocumentId(Uri.parse(tree)) } catch (_: Exception) { return "" }
    val split = id.split(":", limit = 2)
    if (split.size != 2 || split[0] != "primary") return ""
    return "/storage/emulated/0/${split[1]}"
}
