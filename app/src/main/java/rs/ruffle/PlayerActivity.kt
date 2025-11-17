package rs.ruffle

import android.annotation.SuppressLint
import android.content.Intent
import android.content.res.Configuration
import android.graphics.Bitmap
import android.net.Uri
import android.os.Build
import android.os.Build.VERSION_CODES
import android.os.Bundle
import android.os.Environment
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Menu
import android.view.MenuItem
import android.view.MotionEvent
import android.view.PixelCopy
import android.view.View
import android.view.ViewGroup
import android.view.Window
import android.view.WindowManager
import android.widget.Button
import android.widget.PopupMenu
import android.widget.Toast
import androidx.constraintlayout.widget.ConstraintLayout
import androidx.core.graphics.createBitmap
import androidx.core.graphics.get
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.core.view.isVisible
import com.google.androidgamesdk.GameActivity
import java.io.DataInputStream
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

class PlayerActivity : GameActivity() {
    @Suppress("unused")
    // Used by Rust
    private val swfBytes: ByteArray?
        get() {
            val uri = intent.data
            if (uri?.scheme == "content") {
                try {
                    contentResolver.openInputStream(uri).use { inputStream ->
                        if (inputStream == null) {
                            return null
                        }
                        val bytes = ByteArray(inputStream.available())
                        val dataInputStream = DataInputStream(inputStream)
                        dataInputStream.readFully(bytes)
                        return bytes
                    }
                } catch (ignored: IOException) {
                }
            }
            return null
        }

    @Suppress("unused")
    // Used by Rust
    private val swfUri: String?
        get() {
            return intent.dataString
        }

    @Suppress("unused")
    // Used by Rust
    private val traceOutput: String?
        get() {
            return intent.getStringExtra("traceOutput")
        }

    @Suppress("unused")
    // Used by Rust
    private fun navigateToUrl(url: String?) {
//        startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
    }

    private var loc = IntArray(2)

    @Suppress("unused")
    // Handle of an EventLoopProxy over in rust-land
    private val eventLoopHandle: Long = 0

    @Suppress("unused")
    // Used by Rust
    private val locInWindow: IntArray
        get() {
            mSurfaceView.getLocationInWindow(loc)
            return loc
        }

    @Suppress("unused")
    // Used by Rust
    private val surfaceWidth: Int
        get() = mSurfaceView.width

    @Suppress("unused")
    // Used by Rust
    private val surfaceHeight: Int
        get() = mSurfaceView.height

    private external fun keydown(keyTag: String)
    private external fun keyup(keyTag: String)
    private external fun mousedown(button: Int)
    private external fun mouseup(button: Int)
    private external fun requestContextMenu()
    private external fun runContextMenuCallback(index: Int)
    private external fun clearContextMenu()
    private external fun setMouseMode(mode: Int)
    private external fun setBackendMode(mode: Int)
    
    // Mouse mode: 0 = Direct Touch, 1 = Relative Swipe
    private var mouseMode = 0
    
    // Backend mode: 0 = VULKAN (default), 1 = GL
    private var backendMode = 0

    @Suppress("unused")
    // Used by Rust
    private fun showContextMenu(items: Array<String>) {
        runOnUiThread {
            val popup = PopupMenu(this, findViewById(R.id.button_cm))
            val menu = popup.menu
            if (Build.VERSION.SDK_INT >= VERSION_CODES.P) {
                menu.setGroupDividerEnabled(true)
            }
            var group = 1
            for (i in items.indices) {
                val elements = items[i].split(" ".toRegex(), limit = 4).toTypedArray()
                val enabled = elements[0].toBoolean()
                val separatorBefore = elements[1].toBoolean()
                val checked = elements[2].toBoolean()
                val caption = elements[3]
                if (separatorBefore) group += 1
                val item = menu.add(group, i, Menu.NONE, caption)
                item.isEnabled = enabled
                if (checked) {
                    item.isCheckable = true
                    item.isChecked = true
                }
            }
            val exitItemId: Int = items.size
            menu.add(group, exitItemId, Menu.NONE, "Exit")
            popup.setOnMenuItemClickListener { item: MenuItem ->
                if (item.itemId == exitItemId) {
                    finish()
                } else {
                    runContextMenuCallback(item.itemId)
                }
                true
            }
            popup.setOnDismissListener { clearContextMenu() }
            popup.show()
        }
    }

    @Suppress("unused")
    // Used by Rust
    private fun getAndroidDataStorageDir(): String {
        // TODO It can also be placed in an external storage path in the future to share archived content
        val storageDirPath = "${filesDir.absolutePath}/ruffle/shared_objects"
        val storageDir = File(storageDirPath)
        if (!storageDir.exists()) {
            storageDir.mkdirs()
        }
        return storageDirPath
    }

    @Suppress("unused")
    // Called when content is ready to be interacted with
    private fun onContentReady() {
        Log.i("ruffle", "Content is ready!")
    }

    /**
     * Remove letterbox (black borders) from screenshot by auto-cropping
     * Returns the cropped bitmap or the original if no letterbox is detected
     */
    private fun removeLetterbox(original: Bitmap): Bitmap {
        val width = original.width
        val height = original.height
        
        // Threshold for considering a pixel as "black" (letterbox)
        // Using a slightly higher threshold to account for compression artifacts
        val blackThreshold = 30
        
        var topCrop = 0
        var bottomCrop = height - 1
        var leftCrop = 0
        var rightCrop = width - 1
        
        // Find top border
        outer@ for (y in 0 until height) {
            for (x in 0 until width) {
                val pixel = original[x, y]
                val r = (pixel shr 16) and 0xFF
                val g = (pixel shr 8) and 0xFF
                val b = pixel and 0xFF
                if (r > blackThreshold || g > blackThreshold || b > blackThreshold) {
                    topCrop = y
                    break@outer
                }
            }
        }
        
        // Find bottom border
        outer@ for (y in height - 1 downTo 0) {
            for (x in 0 until width) {
                val pixel = original[x, y]
                val r = (pixel shr 16) and 0xFF
                val g = (pixel shr 8) and 0xFF
                val b = pixel and 0xFF
                if (r > blackThreshold || g > blackThreshold || b > blackThreshold) {
                    bottomCrop = y
                    break@outer
                }
            }
        }
        
        // Find left border
        outer@ for (x in 0 until width) {
            for (y in topCrop..bottomCrop) {
                val pixel = original[x, y]
                val r = (pixel shr 16) and 0xFF
                val g = (pixel shr 8) and 0xFF
                val b = pixel and 0xFF
                if (r > blackThreshold || g > blackThreshold || b > blackThreshold) {
                    leftCrop = x
                    break@outer
                }
            }
        }
        
        // Find right border
        outer@ for (x in width - 1 downTo 0) {
            for (y in topCrop..bottomCrop) {
                val pixel = original[x, y]
                val r = (pixel shr 16) and 0xFF
                val g = (pixel shr 8) and 0xFF
                val b = pixel and 0xFF
                if (r > blackThreshold || g > blackThreshold || b > blackThreshold) {
                    rightCrop = x
                    break@outer
                }
            }
        }
        
        // Calculate cropped dimensions
        val croppedWidth = rightCrop - leftCrop + 1
        val croppedHeight = bottomCrop - topCrop + 1
        
        // If no significant letterbox detected (less than 5% crop), return original
        val cropPercentage = 1.0 - (croppedWidth * croppedHeight).toDouble() / (width * height).toDouble()
        if (cropPercentage < 0.05) {
            Log.i("ruffle", "No significant letterbox detected, keeping original size")
            return original
        }
        
        Log.i("ruffle", "Removing letterbox: original ${width}x${height}, cropped ${croppedWidth}x${croppedHeight}")
        Log.i("ruffle", "Crop bounds: left=$leftCrop, top=$topCrop, right=$rightCrop, bottom=$bottomCrop")
        
        // Create cropped bitmap
        return Bitmap.createBitmap(original, leftCrop, topCrop, croppedWidth, croppedHeight)
    }

    private fun captureScreenshot() {
        try {
            // Create screenshots directory
            val picturesDir = Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_PICTURES)
            val screenshotsDir = File(picturesDir, "Ruffle")
            if (!screenshotsDir.exists()) {
                screenshotsDir.mkdirs()
            }

            // Generate filename with timestamp
            val timestamp = SimpleDateFormat("yyyyMMdd_HHmmss", Locale.getDefault()).format(Date())
            val filename = "ruffle_screenshot_$timestamp.png"
            val file = File(screenshotsDir, filename)

            // Get the actual game rendering area (excluding letterbox if any)
            val surfaceWidth = mSurfaceView.width
            val surfaceHeight = mSurfaceView.height
            
            Log.i("ruffle", "Capturing screenshot: ${surfaceWidth}x${surfaceHeight}")

            // Use PixelCopy for Android N and above (more reliable)
            // This captures only the visible game area from the Surface
            val bitmap = createBitmap(surfaceWidth, surfaceHeight)

            // PixelCopy captures directly from the Surface,
            // which contains only the game rendering (no UI elements)
            PixelCopy.request(
                mSurfaceView.holder.surface,
                bitmap,
                { copyResult ->
                    if (copyResult == PixelCopy.SUCCESS) {
                        try {
                            // Remove letterbox from screenshot
                            val croppedBitmap = removeLetterbox(bitmap)

                            FileOutputStream(file).use { out ->
                                croppedBitmap.compress(Bitmap.CompressFormat.PNG, 100, out)
                            }

                            // Clean up bitmaps
                            if (croppedBitmap !== bitmap) {
                                croppedBitmap.recycle()
                            }

                            runOnUiThread {
                                Toast.makeText(
                                    this,
                                    "게임 화면이 저장되었습니다: $filename",
                                    Toast.LENGTH_LONG
                                ).show()
                            }
                            Log.i("ruffle", "Game screenshot saved: ${file.absolutePath}")
                        } catch (e: IOException) {
                            Log.e("ruffle", "Failed to save screenshot", e)
                            runOnUiThread {
                                Toast.makeText(
                                    this,
                                    "스크린샷 저장 실패: ${e.message}",
                                    Toast.LENGTH_SHORT
                                ).show()
                            }
                        } catch (e: Exception) {
                            Log.e("ruffle", "Failed to crop screenshot", e)
                            // Fall back to saving original bitmap
                            try {
                                FileOutputStream(file).use { out ->
                                    bitmap.compress(Bitmap.CompressFormat.PNG, 100, out)
                                }
                                runOnUiThread {
                                    Toast.makeText(
                                        this,
                                        "게임 화면이 저장되었습니다 (원본): $filename",
                                        Toast.LENGTH_LONG
                                    ).show()
                                }
                            } catch (e2: IOException) {
                                Log.e("ruffle", "Failed to save fallback screenshot", e2)
                            }
                        }
                    } else {
                        Log.e("ruffle", "PixelCopy failed: $copyResult")
                        runOnUiThread {
                            Toast.makeText(
                                this,
                                "스크린샷 캡처 실패",
                                Toast.LENGTH_SHORT
                            ).show()
                        }
                    }
                    bitmap.recycle()
                },
                Handler(Looper.getMainLooper())
            )
        } catch (e: Exception) {
            Log.e("ruffle", "Screenshot capture error", e)
            Toast.makeText(
                this,
                "스크린샷 오류: ${e.message}",
                Toast.LENGTH_SHORT
            ).show()
        }
    }

    override fun onCreateSurfaceView() {
        val inflater = layoutInflater

        @SuppressLint("InflateParams")
        val layout = inflater.inflate(R.layout.keyboard, null) as ConstraintLayout

        contentViewId = View.generateViewId()
        layout.id = contentViewId
        setContentView(layout)
        mSurfaceView = InputEnabledSurfaceView(this)

        mSurfaceView.contentDescription = "Ruffle Player"

        val placeholder = findViewById<View>(R.id.placeholder)
        val pars = placeholder.layoutParams as ConstraintLayout.LayoutParams
        val parent = placeholder.parent as ViewGroup
        val index = parent.indexOfChild(placeholder)
        parent.removeView(placeholder)
        parent.addView(mSurfaceView, index)
        mSurfaceView.setLayoutParams(pars)
        val keys = gatherAllDescendantsOfType<Button>(
            layout.getViewById(R.id.keyboard),
            Button::class.java
        )
        for (b in keys) {
            b.setOnTouchListener { view: View, motionEvent: MotionEvent ->
                val tag = view.tag as String
                if (motionEvent.action == MotionEvent.ACTION_DOWN) keydown(tag)
                if (motionEvent.action == MotionEvent.ACTION_UP) keyup(tag)
                view.performClick()
                true  // 이벤트를 소비하여 게임 화면으로 전파되지 않도록 함
            }
        }
        layout.findViewById<View>(R.id.button_kb).setOnClickListener {
            val keyboard = layout.getViewById(R.id.keyboard)
            if (keyboard.isVisible) {
                keyboard.visibility = View.GONE
            } else {
                keyboard.visibility = View.VISIBLE
            }
        }
        layout.findViewById<View>(R.id.button_cm)
            .setOnClickListener { requestContextMenu() }
        
        // Mouse mode toggle button
        val mouseModeButton = layout.findViewById<Button>(R.id.button_mouse_mode)
        mouseModeButton.setOnClickListener {
            mouseMode = if (mouseMode == 0) 1 else 0
            setMouseMode(mouseMode)
            mouseModeButton.text = if (mouseMode == 0) "🖱" else "👆"
            Log.i("ruffle", "Mouse mode changed to: ${if (mouseMode == 0) "Direct Touch" else "Relative Swipe"}")
        }
        
        // Screenshot button
        layout.findViewById<Button>(R.id.button_screenshot).setOnClickListener {
            captureScreenshot()
            Log.i("ruffle", "Screenshot requested")
        }
        
        // Backend mode toggle button
        val backendButton = layout.findViewById<Button>(R.id.button_backend)
        backendButton.setOnClickListener {
            backendMode = if (backendMode == 0) 1 else 0
            setBackendMode(backendMode)
            backendButton.text = if (backendMode == 0) "🖥" else "🔧"
            val backendName = if (backendMode == 0) "VULKAN" else "GL"
            Toast.makeText(this, "백엔드: $backendName (재시작 필요)", Toast.LENGTH_SHORT).show()
            Log.i("ruffle", "Backend mode changed to: $backendName")
        }
        
        layout.requestLayout()
        layout.requestFocus()
        mSurfaceView.holder.addCallback(this)
        ViewCompat.setOnApplyWindowInsetsListener(mSurfaceView, this)
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        val keyboard = findViewById<View>(R.id.keyboard)
        val isLandscape = newConfig.orientation == Configuration.ORIENTATION_LANDSCAPE
        keyboard.visibility = if (isLandscape) View.GONE else View.VISIBLE
    }

    private fun hideSystemUI() {
        // This will put the game behind any cutouts and waterfalls on devices which have
        // them, so the corresponding insets will be non-zero.
        if (Build.VERSION.SDK_INT >= VERSION_CODES.R) {
            window.attributes.layoutInDisplayCutoutMode =
                WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS
        }
        // From API 30 onwards, this is the recommended way to hide the system UI, rather than
        // using View.setSystemUiVisibility.
        val decorView = window.decorView
        val controller = WindowInsetsControllerCompat(
            window,
            decorView
        )
        controller.hide(WindowInsetsCompat.Type.systemBars())
        controller.hide(WindowInsetsCompat.Type.displayCutout())
        controller.systemBarsBehavior =
            WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        nativeInit { message ->
            Log.e("ruffle", "Handling panic: $message")
            startActivity(
                Intent(this, PanicActivity::class.java).apply {
                    putExtra("message", message)
                }
            )
        }
        
        // Set default backend mode: 0 = VULKAN (default), 1 = GL
        setBackendMode(backendMode)
        Log.i("ruffle", "Initial backend mode set to: ${if (backendMode == 0) "VULKAN" else "GL"}")
        
        // When true, the app will fit inside any system UI windows.
        // When false, we render behind any system UI windows.
        WindowCompat.setDecorFitsSystemWindows(window, false)
        hideSystemUI()
        // You can set IME fields here or in native code using GameActivity_setImeEditorInfoFields.
        // We set the fields in native_engine.cpp.
        // super.setImeEditorInfoFields(InputType.TYPE_CLASS_TEXT,
        //     IME_ACTION_NONE, IME_FLAG_NO_FULLSCREEN );
        requestNoStatusBarFeature()
        supportActionBar?.hide()
        super.onCreate(savedInstanceState)
    }

    // Used by Rust
    @Suppress("unused")
    val isGooglePlayGames: Boolean
        get() {
            val pm = packageManager
            return pm.hasSystemFeature("com.google.android.play.feature.HPE_EXPERIENCE")
        }

    private fun requestNoStatusBarFeature() {
        // Hiding the status bar this way makes it see through when pulled down
        requestWindowFeature(Window.FEATURE_NO_TITLE)
        WindowInsetsControllerCompat(
            window,
            mSurfaceView
        ).hide(WindowInsetsCompat.Type.statusBars())
    }

    companion object {
        // Mouse button constants
        const val MOUSE_BUTTON_LEFT = 0
        const val MOUSE_BUTTON_RIGHT = 1
        const val MOUSE_BUTTON_MIDDLE = 2

        init {
            System.loadLibrary("ruffle_android")
        }

        @JvmStatic
        private external fun nativeInit(crashCallback: CrashCallback)

        private fun <T> gatherAllDescendantsOfType(v: View, t: Class<*>): List<T> {
            val result: MutableList<T> = ArrayList()
            @Suppress("UNCHECKED_CAST")
            if (t.isInstance(v)) result.add(v as T)
            if (v is ViewGroup) {
                for (i in 0 until v.childCount) {
                    result.addAll(gatherAllDescendantsOfType(v.getChildAt(i), t))
                }
            }
            return result
        }
    }

    fun interface CrashCallback {
        fun onCrash(message: String)
    }
}
