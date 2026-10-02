package org.nativecod.app

import android.app.Application
import com.facebook.react.PackageList
import com.facebook.react.ReactApplication
import com.facebook.react.ReactHost
import com.facebook.react.defaults.DefaultReactHost.getDefaultReactHost

class MainApplication : Application(), ReactApplication {
    override val reactHost: ReactHost by lazy {
        check(Application.getProcessName() == packageName)
        getDefaultReactHost(context = applicationContext,
            packageList = PackageList(this).packages.apply { add(CodPackage()) },
            useDevSupport = false)
    }
    override fun onCreate() {
        super.onCreate()
    }
}
