// OyNIx native glue: the few things that must be done from C++ because Qt
// only exposes them there (graphics API selection before the GUI starts,
// Qt WebEngine initialisation, and the network request interceptor).
#pragma once

#include <cstdint>
#include "rust/cxx.h"

class QObject;

namespace oynix {

// Choose the Qt Quick RHI backend. 1=Vulkan 2=OpenGL ES 3=OpenGL 4=Software.
void prepare_graphics(int32_t backend);

// Must run before QGuiApplication is constructed.
void init_webengine();

// After QGuiApplication exists: application name, icon, desktop file name.
void set_app_metadata(rust::Str profile);

// Attach the content blocker / privacy interceptor to a profile created in
// QML. Accepts any QObject*; non-profiles are ignored.
void attach_interceptor(QObject *profile);

// Actual graphics API Qt Quick ended up using (for the About page).
rust::String active_graphics_api();

// Colour scheme websites see via prefers-color-scheme (and Qt widgets/dialogs):
// 0 = follow the system, 1 = light, 2 = dark.
void set_color_scheme(int32_t scheme);

// Real input for the browser agent, delivered to a QQuickItem (the web view)
// through its window, exactly like physical input. Coordinates are item-local.
// mouse kind: 0 = move, 1 = click. button: 1 left, 2 right, 4 middle.
void send_mouse(QObject *item, double x, double y, int32_t button, int32_t clicks, int32_t modifiers, int32_t kind);
void send_key(QObject *item, int32_t key, int32_t modifiers, rust::Str text);
void send_text(QObject *item, rust::Str text);
void send_wheel(QObject *item, double x, double y, double dx, double dy);

// Qt's qwebchannel.js client library (from the Qt WebChannel resources).
rust::String webchannel_js();

// qVersion() of the Qt libraries actually loaded.
rust::String qt_runtime_version();

} // namespace oynix
