#include "oynix/cpp/native.h"
#include "oynix/src/bridge/native.cxx.h"

#include <QtCore/QByteArray>
#include <QtCore/QCoreApplication>
#include <QtCore/QFile>
#include <QtCore/QString>
#include <QtCore/QUrl>
#include <QtCore/QtGlobal>
#include <QtGui/QGuiApplication>
#include <QtNetwork/QNetworkProxyFactory>
#include <QtGui/QIcon>
#include <QtGui/QStyleHints>
#include <QtGui/QSurfaceFormat>
#include <QtGui/QInputMethodEvent>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPointingDevice>
#include <QtGui/QWheelEvent>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <QtQuick/QSGRendererInterface>
#include <QtWebEngineCore/QWebEngineCookieStore>
#include <QtWebEngineCore/QWebEngineUrlRequestInterceptor>
#include <QtWebEngineCore/QWebEngineUrlRequestInfo>
#include <QtWebEngineQuick/QQuickWebEngineProfile>
#include <QtWebEngineQuick/QtWebEngineQuick>

namespace oynix {

namespace {

static rust::Str to_str(const QByteArray &b) { return rust::Str(b.constData(), static_cast<size_t>(b.size())); }

class Interceptor final : public QWebEngineUrlRequestInterceptor {
public:
    using QWebEngineUrlRequestInterceptor::QWebEngineUrlRequestInterceptor;

    void interceptRequest(QWebEngineUrlRequestInfo &info) override {
        const QByteArray url = info.requestUrl().toEncoded();
        const QByteArray first = info.firstPartyUrl().toEncoded();
        const int type = static_cast<int>(info.resourceType());
        const uint32_t flags = privacy_flags();

        // Main-frame navigations: strip tracking parameters.
        if (type == QWebEngineUrlRequestInfo::ResourceTypeMainFrame) {
            if (flags & 4u) {
                rust::String cleaned = strip_tracking(to_str(url));
                if (cleaned.size() != static_cast<size_t>(url.size())) {
                    info.redirect(QUrl::fromEncoded(QByteArray(cleaned.data(), static_cast<qsizetype>(cleaned.size()))));
                    return;
                }
            }
        } else if (should_block(to_str(url), to_str(first), type)) {
            info.block(true);
            return;
        }
        if (flags & 1u) info.setHttpHeader("DNT", "1");
        if (flags & 2u) info.setHttpHeader("Sec-GPC", "1");
    }
};

Interceptor *g_interceptor = nullptr;

Interceptor *interceptor() {
    if (!g_interceptor) g_interceptor = new Interceptor(QCoreApplication::instance());
    return g_interceptor;
}

} // namespace

void prepare_graphics(int32_t backend) {
    switch (backend) {
    case 1:
        QQuickWindow::setGraphicsApi(QSGRendererInterface::Vulkan);
        break;
    case 2: {
        QSurfaceFormat fmt = QSurfaceFormat::defaultFormat();
        fmt.setRenderableType(QSurfaceFormat::OpenGLES);
        fmt.setVersion(3, 0);
        QSurfaceFormat::setDefaultFormat(fmt);
        QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);
        break;
    }
    case 3:
        QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);
        break;
    default:
        QQuickWindow::setGraphicsApi(QSGRendererInterface::Software);
        break;
    }
}

void init_webengine() {
    QCoreApplication::setAttribute(Qt::AA_ShareOpenGLContexts);
    QCoreApplication::setOrganizationName(QStringLiteral("OyNIx"));
    QCoreApplication::setApplicationName(QStringLiteral("OyNIx"));
    QtWebEngineQuick::initialize();
}

void set_app_metadata(rust::Str profile) {
    QGuiApplication::setApplicationDisplayName(QStringLiteral("OyNIx"));
    QGuiApplication::setDesktopFileName(QStringLiteral("oynix"));
    // Filter-list downloads etc. honour the system / environment proxy,
    // like the web views do.
    QNetworkProxyFactory::setUseSystemConfiguration(true);
    QGuiApplication::setWindowIcon(QIcon::fromTheme(QStringLiteral("oynix"),
                                                    QIcon(QStringLiteral(":/qt/qml/OyNIx/assets/icon.png"))));
    Q_UNUSED(profile);
}

static void install(QQuickWebEngineProfile *p) {
    p->setUrlRequestInterceptor(interceptor());
    // Third-party cookie blocking (bit 3 of the privacy flags).
    if (QWebEngineCookieStore *store = p->cookieStore()) {
        store->setCookieFilter([](const QWebEngineCookieStore::FilterRequest &req) {
            return !((privacy_flags() & 8u) && req.thirdParty);
        });
    }
}

void attach_interceptor(QObject *obj) {
    if (auto *p = qobject_cast<QQuickWebEngineProfile *>(obj)) install(p);
}

rust::String active_graphics_api() {
    switch (QQuickWindow::graphicsApi()) {
    case QSGRendererInterface::Vulkan: return rust::String("Vulkan");
    case QSGRendererInterface::OpenGL:
        return rust::String(QSurfaceFormat::defaultFormat().renderableType() == QSurfaceFormat::OpenGLES ? "OpenGL ES" : "OpenGL");
    case QSGRendererInterface::Software: return rust::String("Software");
    default: return rust::String("Other");
    }
}

static QString qstr(rust::Str s) { return QString::fromUtf8(s.data(), static_cast<qsizetype>(s.size())); }

void send_mouse(QObject *obj, double x, double y, int32_t button, int32_t clicks, int32_t modifiers, int32_t kind) {
    auto *item = qobject_cast<QQuickItem *>(obj);
    if (!item || !item->window()) return;
    QQuickWindow *w = item->window();
    const QPointF scene = item->mapToScene(QPointF(x, y));
    const QPointF global = w->mapToGlobal(scene);
    const auto mods = Qt::KeyboardModifiers(modifiers);
    const Qt::MouseButton b = button == 2 ? Qt::RightButton : button == 4 ? Qt::MiddleButton : Qt::LeftButton;
    auto post = [&](QEvent::Type t, Qt::MouseButton btn, Qt::MouseButtons held) {
        QMouseEvent ev(t, scene, scene, global, btn, held, mods, QPointingDevice::primaryPointingDevice());
        QCoreApplication::sendEvent(w, &ev);
    };
    post(QEvent::MouseMove, Qt::NoButton, Qt::NoButton);
    if (kind == 0) return;
    for (int i = 0; i < std::max(1, int(clicks)); ++i) {
        post(i == 1 ? QEvent::MouseButtonDblClick : QEvent::MouseButtonPress, b, b);
        post(QEvent::MouseButtonRelease, b, Qt::NoButton);
    }
}

void send_key(QObject *obj, int32_t key, int32_t modifiers, rust::Str text) {
    auto *item = qobject_cast<QQuickItem *>(obj);
    if (!item || !item->window()) return;
    item->forceActiveFocus();
    const QString t = qstr(text);
    const auto mods = Qt::KeyboardModifiers(modifiers);
    QKeyEvent press(QEvent::KeyPress, key, mods, t);
    QKeyEvent release(QEvent::KeyRelease, key, mods, t);
    QCoreApplication::sendEvent(item->window(), &press);
    QCoreApplication::sendEvent(item->window(), &release);
}

void send_text(QObject *obj, rust::Str text) {
    auto *item = qobject_cast<QQuickItem *>(obj);
    if (!item || !item->window()) return;
    item->forceActiveFocus();
    QObject *target = item->window()->focusObject();
    if (!target) target = item;
    // Commit text through the input method so any Unicode (accents, emoji,
    // CJK) arrives as typed text, as an on-screen keyboard would deliver it.
    QInputMethodEvent ev;
    ev.setCommitString(qstr(text));
    QCoreApplication::sendEvent(target, &ev);
}

void send_wheel(QObject *obj, double x, double y, double dx, double dy) {
    auto *item = qobject_cast<QQuickItem *>(obj);
    if (!item || !item->window()) return;
    QQuickWindow *w = item->window();
    const QPointF scene = item->mapToScene(QPointF(x, y));
    // 120 angle units ≈ one notch; dx/dy are in pixels (positive = down/right).
    const QPoint angle(int(-dx * 1.2), int(-dy * 1.2));
    QWheelEvent ev(scene, w->mapToGlobal(scene), QPoint(), angle, Qt::NoButton, Qt::NoModifier, Qt::NoScrollPhase, false);
    QCoreApplication::sendEvent(w, &ev);
}

void set_color_scheme(int32_t scheme) {
    QStyleHints *hints = QGuiApplication::styleHints();
    if (!hints) return;
    switch (scheme) {
    case 1: hints->setColorScheme(Qt::ColorScheme::Light); break;
    case 2: hints->setColorScheme(Qt::ColorScheme::Dark); break;
    default: hints->unsetColorScheme(); break;
    }
    if (qEnvironmentVariableIsSet("OYNIX_DEBUG_SCHEME")) qWarning("oynix: requested scheme %d, styleHints now %d", int(scheme), int(hints->colorScheme()));
}

rust::String webchannel_js() {
    QFile f(QStringLiteral(":/qtwebchannel/qwebchannel.js"));
    if (!f.open(QIODevice::ReadOnly)) return rust::String();
    const QByteArray b = f.readAll();
    return rust::String(b.constData(), static_cast<size_t>(b.size()));
}

rust::String qt_runtime_version() { return rust::String(qVersion()); }

} // namespace oynix
