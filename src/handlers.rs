pub mod easy_gnome {
    use std::collections::HashMap;
    use zbus::{dbus_proxy, Result};

    #[dbus_proxy(
        interface = "org.freedesktop.login1.Manager",
        default_service = "org.freedesktop.login1",
        default_path = "/org/freedesktop/login1"
    )]
    trait PowerManagement {
        async fn Suspend(&self, arg: bool) -> Result<()>;
        async fn PowerOff(&self, arg: bool) -> Result<()>;
        async fn Reboot(&self, arg: bool) -> Result<()>;
    }

    #[derive(PartialEq, Debug, Clone, Copy)]
    pub enum PowerProfile {
        PowerSaver,
        Balanced,
        Performance,
    }

    impl PowerProfile {
        fn as_str(&self) -> &'static str {
            match self {
                PowerProfile::PowerSaver => "power-saver",
                PowerProfile::Balanced => "balanced",
                PowerProfile::Performance => "performance",
            }
        }
        fn from(profile: &str) -> PowerProfile {
            match profile {
                "power-saver" => PowerProfile::PowerSaver,
                "balanced" => PowerProfile::Balanced,
                "performance" => PowerProfile::Performance,
                _ => PowerProfile::Balanced,
            }
        }
    }
    #[dbus_proxy(
        interface = "net.hadess.PowerProfiles",
        default_service = "net.hadess.PowerProfiles",
        default_path = "/net/hadess/PowerProfiles"
    )]
    trait PowerProfiles {
        #[dbus_proxy(property)]
        fn ActiveProfile(&self) -> Result<String>;
        #[dbus_proxy(property)]
        fn set_ActiveProfile(&self, profile: String) -> Result<()>;
    }

    // Shell extensions
    #[dbus_proxy(
        interface = "org.gnome.Shell.Extensions",
        default_service = "org.gnome.Shell.Extensions",
        default_path = "/org/gnome/Shell/Extensions"
    )]
    trait Extensions {
        async fn ListExtensions(
            &self,
        ) -> Result<HashMap<String, HashMap<String, zvariant::OwnedValue>>>;
        async fn LaunchExtensionPrefs(&self, uuid: String) -> Result<()>;
        async fn EnableExtension(&self, uuid: String) -> Result<bool>;
        async fn DisableExtension(&self, uuid: String) -> Result<bool>;
        async fn UninstallExtension(&self, uuid: String) -> Result<bool>;
    }

    /// # Extension states
    /// ```xml
    /// <member>1: ENABLED</member>
    /// <member>2: DISABLED</member>
    /// <member>3: ERROR</member>
    /// <member>4: OUT_OF_DATE</member>
    /// <member>5: DOWNLOADING</member>
    /// <member>6: INITIALIZED</member>
    /// <member>99: UNINSTALLED</member>
    /// ```
    /// https://gitlab.gnome.org/GNOME/gnome-shell/-/blob/92d3c6e051958b31151bf9538205a71cab6f70d7/data/dbus-interfaces/org.gnome.Shell.Extensions.xml#L73
    #[derive(Debug)]
    pub enum ListExtensionState {
        ENABLED = 1,
        DISABLED = 2,
        ERROR = 3,
        OUT_OF_DATE = 4,
        DOWNLOADING = 5,
        INITIALIZED = 6,
        UNINSTALLED = 99,
    }
    #[derive(Debug)]
    pub struct ListExtension {
        pub uuid: String,
        pub name: String,
        pub description: String,
        pub state: ListExtensionState,
        pub version: String,
        pub url: String,
    }
    impl ExtensionsProxy<'static> {
        async fn launch_extension_prefs(&self, uuid: &str) -> Result<()> {
            let _reply = self
                .LaunchExtensionPrefs(uuid.to_string())
                .await
                .unwrap_or_else(|_| ());
            Ok(())
        }
        async fn list_extensions(&self) -> Vec<ListExtension> {
            let list = self.ListExtensions().await.unwrap();
            let mut list_extension: Vec<ListExtension> = Vec::new();
            for extension in list {
                let uuid = extension.0;
                let name = extension
                    .1
                    .get("name")
                    .unwrap()
                    .to_owned()
                    .try_into()
                    .unwrap();
                let description = extension
                    .1
                    .get("description")
                    .unwrap()
                    .to_owned()
                    .try_into()
                    .unwrap();
                let try_version = extension.1.get("version");
                let version = match try_version {
                    Some(version) => version
                        .to_owned()
                        .try_into()
                        .unwrap_or_else(|_| "".to_string()),
                    None => "".to_string(),
                };
                let state_number: f64 = extension
                    .1
                    .get("state")
                    .unwrap()
                    .to_owned()
                    .try_into()
                    .unwrap();
                let state: ListExtensionState = match state_number {
                    1.0 => ListExtensionState::ENABLED,
                    2.0 => ListExtensionState::DISABLED,
                    3.0 => ListExtensionState::ERROR,
                    4.0 => ListExtensionState::OUT_OF_DATE,
                    5.0 => ListExtensionState::DOWNLOADING,
                    6.0 => ListExtensionState::INITIALIZED,
                    99.0 => ListExtensionState::UNINSTALLED,
                    _ => ListExtensionState::UNINSTALLED,
                };
                let url = extension
                    .1
                    .get("url")
                    .unwrap()
                    .to_owned()
                    .try_into()
                    .unwrap();
                let item = ListExtension {
                    uuid,
                    name,
                    description,
                    version,
                    state,
                    url,
                };
                list_extension.push(item);
            }
            list_extension
        }
    }
    // Shell screenshot
    #[dbus_proxy(
        interface = "org.gnome.Shell.Screenshot",
        default_path = "/org/gnome/Shell/Screenshot"
    )]
    trait Screenshot {
        async fn PickColor(&self) -> Result<HashMap<String, zvariant::OwnedValue>>;
    }
    impl ScreenshotProxy<'static> {
        async fn pick_color(&self) -> (f64, f64, f64) {
            let pick_color = self.PickColor().await.unwrap();
            let value = pick_color.get("color").unwrap();
            let (r, g, b): (f64, f64, f64) = value.to_owned().try_into().unwrap();

            (r, g, b)
        }
    }

    #[dbus_proxy(
        interface = "org.gnome.SettingsDaemon.Power.Screen",
        default_service = "org.gnome.SettingsDaemon.Power",
        default_path = "/org/gnome/SettingsDaemon/Power"
    )]
    trait Screen {
        #[dbus_proxy(property)]
        fn Brightness(&self) -> Result<i32>;
        #[dbus_proxy(property)]
        fn set_Brightness(&self, brightness: i32) -> Result<()>;
        fn StepUp(&self) -> Result<()>;
        fn StepDown(&self) -> Result<()>;
    }

    pub mod power {
        use zbus::Connection;

        use crate::handlers::easy_gnome::PowerManagementProxy;

        use super::{PowerProfile, PowerProfilesProxy};

        pub async fn power_off() {
            let connection = Connection::system().await.unwrap();
            let proxy = PowerManagementProxy::new(&connection).await.unwrap();
            proxy.PowerOff(true).await.unwrap();
        }
        pub async fn suspend() {
            let connection = Connection::system().await.unwrap();
            let proxy = PowerManagementProxy::new(&connection).await.unwrap();
            proxy.Suspend(true).await.unwrap();
        }
        pub async fn reboot() {
            let connection = Connection::system().await.unwrap();
            let proxy = PowerManagementProxy::new(&connection).await.unwrap();
            proxy.Reboot(true).await.unwrap();
        }
        pub async fn get_power_profile() -> PowerProfile {
            let connection = Connection::system().await.unwrap();
            let proxy = PowerProfilesProxy::new(&connection).await.unwrap();
            PowerProfile::from(proxy.ActiveProfile().await.unwrap().as_str())
        }
        pub async fn set_power_profile(profile: PowerProfile) {
            let connection = Connection::system().await.unwrap();
            let proxy = PowerProfilesProxy::new(&connection).await.unwrap();
            proxy
                .set_ActiveProfile(profile.as_str().to_string())
                .await
                .unwrap();
        }
    }

    pub mod screenshot {
        use zbus::Connection;

        use crate::handlers::easy_gnome::ScreenshotProxy;

        pub async fn pick_color() -> (f64, f64, f64) {
            let connection = Connection::session().await.unwrap();
            let proxy = ScreenshotProxy::new(&connection).await.unwrap();
            proxy.pick_color().await
        }
    }

    pub mod screen {
        use futures_util::StreamExt;
        use zbus::{Connection, Proxy};

        use crate::handlers::easy_gnome::ScreenProxy;

        pub async fn brightness() -> i32 {
            let connection = Connection::session().await.unwrap();
            let proxy = ScreenProxy::new(&connection).await.unwrap();
            proxy.Brightness().await.unwrap()
        }
        pub async fn set_brightness(brightness: i32) {
            let connection = Connection::session().await.unwrap();
            let proxy = ScreenProxy::new(&connection).await.unwrap();
            proxy.set_Brightness(brightness).await.unwrap();
        }
        pub async fn step_up() {
            let connection = Connection::session().await.unwrap();
            let proxy = ScreenProxy::new(&connection).await.unwrap();
            proxy.StepUp().await.unwrap();
        }
        pub async fn step_down() {
            let connection = Connection::session().await.unwrap();
            let proxy = ScreenProxy::new(&connection).await.unwrap();
            proxy.StepDown().await.unwrap();
        }
        pub async fn on_brightness_changed<F: Fn(i32) + 'static>(callback: F) {
            let connection = Connection::session().await.unwrap();
            let proxy = ScreenProxy::new(&connection).await.unwrap();
            let mut stream = proxy.receive_Brightness_changed().await;
            while let Some(changed) = stream.next().await {
                match changed.get().await {
                    Ok(new_brightness) => {
                        callback(new_brightness);
                    }
                    Err(e) => {
                        eprintln!("Error getting brightness value: {}", e);
                    }
                }
            }
        }
    }

    pub mod nightlight {

        pub fn get_nightlight_active() -> bool {
            crate::dconf::get(
                "org.gnome.settings-daemon.plugins.color",
                "night-light-enabled",
            )
            .unwrap()
            .parse::<bool>()
            .unwrap()
        }
        pub fn set_nightlight_active(active: bool) {
            crate::dconf::set(
                "org.gnome.settings-daemon.plugins.color",
                "night-light-enabled",
                active.to_string().as_str(),
            )
            .unwrap();
        }
        pub fn get_temperature() -> u32 {
            crate::dconf::get(
                "org.gnome.settings-daemon.plugins.color",
                "night-light-temperature",
            )
            .unwrap()
            .parse::<u32>()
            .unwrap()
        }
        pub fn reset_temperature() {
            crate::dconf::reset(
                "org.gnome.settings-daemon.plugins.color",
                "night-light-temperature",
            )
            .unwrap();
        }
        pub fn set_temperature(temperature: u32) {
            crate::dconf::set(
                "org.gnome.settings-daemon.plugins.color",
                "night-light-temperature",
                temperature.to_string().as_str(),
            )
            .unwrap();
        }
    }

    pub mod apps {

        use std::io::Cursor;
        use std::path::PathBuf;

        use gio::glib::{home_dir, GString};
        use gio::prelude::*;
        use gio::AppInfo;
        use gtk::IconTheme;
        use gtk::{prelude::*, IconLookupFlags};
        use image::ImageOutputFormat;

        pub struct App {
            pub name: GString,
            pub description: Option<GString>,
            pub icon: Option<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>,
            pub executable: PathBuf,
        }
        impl App {
            pub fn get_name(&self) -> &GString {
                &self.name
            }
            pub fn get_description(&self) -> &Option<GString> {
                &self.description
            }
            pub fn get_icon(&self) -> &Option<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> {
                &self.icon
            }
            pub fn get_base64_icon(&self) -> Option<String> {
                match &self.icon {
                    Some(icon) => {
                        let mut image_data: Vec<u8> = Vec::new();
                        icon.write_to(&mut Cursor::new(&mut image_data), ImageOutputFormat::Png)
                            .unwrap();
                        let res_base64 = base64::encode(image_data);
                        Some(format!("data:image/png;base64,{}", res_base64))
                    }
                    None => None,
                }
            }
            pub fn launch(&self) -> Result<(), gio::glib::Error> {
                // Find app by name
                let __apps = AppInfo::all();
                __apps
                    .iter()
                    .find(|app| app.name().eq_ignore_ascii_case(&self.name))
                    .unwrap()
                    .launch(&[], None::<&gio::AppLaunchContext>)
            }
        }

        pub struct Apps {
            pub apps: Vec<App>,
        }

        impl Apps {
            pub fn get_apps(&self) -> &Vec<App> {
                &self.apps
            }

            pub fn new() -> Apps {
                const ICON_SIZE: i32 = 128;

                let __apps = AppInfo::all();
                let icon_theme: IconTheme = IconTheme::default().unwrap();
                icon_theme.add_resource_path(
                    format!(
                        "{}/.local/share/icons/hicolor",
                        home_dir().to_str().unwrap()
                    )
                    .as_str(),
                );
                let mut apps: Vec<App> = Vec::new();

                for app in &__apps {
                    if !app.should_show() {
                        continue;
                    }
                    let name = app.name();
                    let description = app.description();
                    let icon = app.icon();
                    let executable = app.executable();

                    if icon.is_none() {
                        apps.push(App {
                            name,
                            description,
                            icon: None,
                            executable,
                        });
                        continue;
                    }
                    let icon_name = gio::prelude::IconExt::to_string(&icon.unwrap()).unwrap();
                    // // Transform icon name to pixbuf
                    let pixbuf = icon_theme
                        .load_icon(&icon_name, ICON_SIZE, IconLookupFlags::GENERIC_FALLBACK)
                        .unwrap_or(
                            icon_theme
                                .load_icon("info", ICON_SIZE, IconLookupFlags::GENERIC_FALLBACK)
                                .unwrap(),
                        );

                    // Pix buf are cuadruplets of u8 (rgba)
                    let bytes: Vec<u8> = pixbuf.unwrap().read_pixel_bytes().to_vec();

                    // Using image library build a png based on cuadruplets (rgba)
                    let png: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
                        match image::RgbaImage::from_vec(ICON_SIZE as u32, ICON_SIZE as u32, bytes)
                        {
                            Some(png) => png,
                            None => continue,
                        };

                    apps.push(App {
                        name,
                        description,
                        icon: Some(png),
                        executable,
                    });
                }
                Apps { apps }
            }
        }
    }

    pub mod battery {
        use upower_dbus::{DeviceProxy, UPowerProxy};

        // Get devices with battery stats
        pub async fn get_current_device_battery() -> zbus::Result<DeviceProxy<'static>> {
            let connection = zbus::Connection::system().await?;
            let upower = UPowerProxy::new(&connection).await?;
            let device: DeviceProxy<'_> = upower.get_display_device().await?;
            Ok(device)
        }
        pub async fn get_devices_battery() -> zbus::Result<Vec<DeviceProxy<'static>>> {
            let connection = zbus::Connection::system().await?;
            let upower = UPowerProxy::new(&connection).await?;
            let devices: Vec<zvariant::OwnedObjectPath> = upower.enumerate_devices().await?;

            let mut devices_battery: Vec<DeviceProxy<'_>> = Vec::new();
            for device in devices {
                let device_proxy = DeviceProxy::new(&connection, device).await?;
                let is_rechargable = device_proxy.is_rechargeable().await?;
                if is_rechargable {
                    devices_battery.push(device_proxy);
                }
            }
            Ok(devices_battery)
        }
    }

    pub mod extensions {
        use zbus::Connection;

        use crate::handlers::easy_gnome::ExtensionsProxy;

        use super::ListExtension;

        pub fn set_extensions_active(active: bool) {
            crate::dconf::set(
                "org.gnome.shell",
                "disable-user-extensions",
                active.to_string().as_str(),
            )
            .unwrap();
        }
        pub fn get_extensions_active() -> bool {
            let value = crate::dconf::get("org.gnome.shell", "disable-user-extensions").unwrap();
            value.parse::<bool>().unwrap()
        }
        pub fn reset_extensions_active() {
            crate::dconf::reset("org.gnome.shell", "disable-user-extensions").unwrap();
        }
        pub async fn get_extensions() -> Vec<ListExtension> {
            let connection = Connection::session().await.unwrap();
            let proxy = ExtensionsProxy::new(&connection).await.unwrap();
            proxy.list_extensions().await
        }
        pub async fn disable_extension(uuid: &str) {
            let connection = Connection::session().await.unwrap();
            let proxy = ExtensionsProxy::new(&connection).await.unwrap();
            proxy.DisableExtension(uuid.to_string()).await.unwrap();
        }
        pub async fn enable_extension(uuid: &str) {
            let connection = Connection::session().await.unwrap();
            let proxy = ExtensionsProxy::new(&connection).await.unwrap();
            proxy.EnableExtension(uuid.to_string()).await.unwrap();
        }
        pub async fn uninstall_extension(uuid: &str) {
            let connection = Connection::session().await.unwrap();
            let proxy = ExtensionsProxy::new(&connection).await.unwrap();
            proxy.UninstallExtension(uuid.to_string()).await.unwrap();
        }
        pub async fn open_extension_preferences(uuid: &str) {
            let connection = Connection::session().await.unwrap();
            let proxy = ExtensionsProxy::new(&connection).await.unwrap();
            proxy.launch_extension_prefs(uuid).await.unwrap();
        }
    }

    pub mod interface {
        pub fn set_show_battery_percentage(show: bool) -> Result<(), String> {
            crate::dconf::set(
                "org.gnome.desktop.interface",
                "show-battery-percentage",
                show.to_string().as_str(),
            )
        }
        pub fn get_show_battery_percentage() -> Result<bool, String> {
            let value =
                crate::dconf::get("org.gnome.desktop.interface", "show-battery-percentage")?;
            Ok(value.parse::<bool>().unwrap())
        }
        pub fn reset_show_battery_percentage() -> Result<(), String> {
            crate::dconf::reset("org.gnome.desktop.interface", "show-battery-percentage")
        }
        pub fn set_locate_pointer(enabled: bool) -> Result<(), String> {
            crate::dconf::set(
                "org.gnome.desktop.interface",
                "locate-pointer",
                enabled.to_string().as_str(),
            )
        }
        pub fn get_locate_pointer() -> Result<bool, String> {
            let value = crate::dconf::get("org.gnome.desktop.interface", "locate-pointer")?;
            Ok(value.parse::<bool>().unwrap())
        }
        pub fn reset_locate_pointer() -> Result<(), String> {
            crate::dconf::reset("org.gnome.desktop.interface", "locate-pointer")
        }
        pub fn set_cursor_size(size: u32) -> Result<(), String> {
            crate::dconf::set(
                "org.gnome.desktop.interface",
                "cursor-size",
                size.to_string().as_str(),
            )
        }
        pub fn get_cursor_size() -> Result<u32, String> {
            let value = crate::dconf::get("org.gnome.desktop.interface", "cursor-size")?;
            Ok(value.parse::<u32>().unwrap())
        }
        pub fn reset_cursor_size() -> Result<(), String> {
            crate::dconf::reset("org.gnome.desktop.interface", "cursor-size")
        }
    }

    pub mod peripherals {
        pub fn set_keyboard_press_delay(delay: u32) -> Result<(), String> {
            crate::dconf::set(
                "org.gnome.desktop.peripherals.keyboard",
                "delay",
                String::from(delay.to_string()).as_str(),
            )
        }
        pub fn get_keyboard_press_delay() -> Result<u32, String> {
            let value: String =
                crate::dconf::get("org.gnome.desktop.peripherals.keyboard", "delay")?;
            Ok(value.parse::<u32>().unwrap())
        }
        pub fn reset_keyboard_press_delay() -> Result<(), String> {
            crate::dconf::reset("org.gnome.desktop.peripherals.keyboard", "delay")
        }
        pub fn set_keyboard_repeat_interval(interval: u32) -> Result<(), String> {
            crate::dconf::set(
                "org.gnome.desktop.peripherals.keyboard",
                "repeat-interval",
                String::from(interval.to_string()).as_str(),
            )
        }
        pub fn get_keyboard_repeat_interval() -> Result<u32, String> {
            let value =
                crate::dconf::get("org.gnome.desktop.peripherals.keyboard", "repeat-interval")?;
            Ok(value.parse::<u32>().unwrap())
        }
        pub fn reset_keyboard_repeat_interval() -> Result<(), String> {
            crate::dconf::reset("org.gnome.desktop.peripherals.keyboard", "repeat-interval")
        }
        pub fn set_mouse_natural_scroll(enabled: bool) -> Result<(), String> {
            crate::dconf::set(
                "org.gnome.desktop.peripherals.mouse",
                "natural-scroll",
                String::from(enabled.to_string()).as_str(),
            )
        }
        pub fn get_mouse_natural_scroll() -> Result<bool, String> {
            let value = crate::dconf::get("org.gnome.desktop.peripherals.mouse", "natural-scroll")?;
            Ok(value.parse::<bool>().unwrap())
        }
        pub fn reset_mouse_natural_scroll() -> Result<(), String> {
            crate::dconf::reset("org.gnome.desktop.peripherals.mouse", "natural-scroll")
        }
        pub fn set_touchpad_tap_to_click(enabled: bool) -> Result<(), String> {
            crate::dconf::set(
                "org.gnome.desktop.peripherals.touchpad",
                "tap-to-click",
                String::from(enabled.to_string()).as_str(),
            )
        }
        pub fn get_touchpad_tap_to_click() -> Result<bool, String> {
            let value =
                crate::dconf::get("org.gnome.desktop.peripherals.touchpad", "tap-to-click")?;
            Ok(value.parse::<bool>().unwrap())
        }
        pub fn reset_touchpad_tap_to_click() -> Result<(), String> {
            crate::dconf::reset("org.gnome.desktop.peripherals.touchpad", "tap-to-click")
        }
        pub fn set_two_finger_scroll(enabled: bool) -> Result<(), String> {
            crate::dconf::set(
                "org.gnome.desktop.peripherals.touchpad",
                "two-finger-scrolling-enabled",
                String::from(enabled.to_string()).as_str(),
            )
        }
        pub fn get_two_finger_scroll() -> Result<bool, String> {
            let value = crate::dconf::get(
                "org.gnome.desktop.peripherals.touchpad",
                "two-finger-scrolling-enabled",
            )?;
            Ok(value.parse::<bool>().unwrap())
        }
        pub fn reset_two_finger_scroll() -> Result<(), String> {
            crate::dconf::reset(
                "org.gnome.desktop.peripherals.touchpad",
                "two-finger-scrolling-enabled",
            )
        }
    }
}
