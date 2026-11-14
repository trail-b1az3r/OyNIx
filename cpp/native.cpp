#include "oynix/cpp/native.h"
#include "oynix/src/bridge/native.cxx.h"

#include <QtCore/QByteArray>
#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QLibraryInfo>
#include <QtCore/QString>
#include <QtCore/QUrl>
#include <QtCore/QtGlobal>
#include <QtGui/QGuiApplication>
#include <QtNetwork/QNetworkProxyFactory>
#include <QtNetwork/QNetworkAccessManager>
#include <QtNetwork/QNetworkRequest>
#include <QtQml/QQmlAbstractUrlInterceptor>
#include <QtQml/QQmlComponent>
#include <QtQml/QQmlContext>
#include <QtQml/QQmlEngine>
#include <QtQml/QQmlNetworkAccessManagerFactory>
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

void clear_cookies(QObject *obj) {
    if (auto *p = qobject_cast<QQuickWebEngineProfile *>(obj))
        if (QWebEngineCookieStore *store = p->cookieStore()) store->deleteAllCookies();
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


// ── Extension panels (graphics.qt) ─────────────────────────────────────

namespace {

// Every request a panel makes fails: XMLHttpRequest, remote images, fonts.
class DenyAllNam : public QNetworkAccessManager {
public:
    using QNetworkAccessManager::QNetworkAccessManager;
protected:
    QNetworkReply *createRequest(Operation op, const QNetworkRequest &req, QIODevice *data) override {
        QNetworkRequest denied(req);
        denied.setUrl(QUrl(QStringLiteral("oynix-denied:")));
        return QNetworkAccessManager::createRequest(op, denied, data);
    }
};

class DenyAllNamFactory : public QQmlNetworkAccessManagerFactory {
public:
    QNetworkAccessManager *create(QObject *parent) override { return new DenyAllNam(parent); }
};

// Allow Qt's own visual modules and the extension's own files; refuse the
// rest (OyNIx, web content, files and settings, other folders on disk).
class PanelUrlInterceptor : public QQmlAbstractUrlInterceptor {
public:
    explicit PanelUrlInterceptor(const QString &dir) : m_dir(QDir::cleanPath(dir) + QLatin1Char('/')) {}
    QUrl intercept(const QUrl &url, DataType type) override {
        const QString s = url.toString();
        // The only QML document from the extension is graphics.qt itself.
        if ((type == QmlFile || type == JavaScriptFile) && url.isLocalFile()
            && QDir::cleanPath(url.toLocalFile()).startsWith(m_dir)
            && QDir::cleanPath(url.toLocalFile()) != m_dir + QStringLiteral("graphics.qml"))
            return QUrl();
        static const char *blocked[] = {
            "/OyNIx/", "/QtWebEngine", "/QtWebChannel", "/QtWebView", "/QtCore/", "/Qt/labs/",
            "/QtQuick/Dialogs", "/QtQuick/LocalStorage", "/QtMultimedia", "/QtPositioning",
            "/QtLocation", "/QtNetwork", "/QtTest", "/QtRemoteObjects", "/QtSensors", "/QtBluetooth", "/QtNfc",
        };
        for (const char *b : blocked)
            if (s.contains(QLatin1String(b))) return QUrl();
        if (type == QmldirFile || url.scheme() == QLatin1String("qrc") || url.scheme() == QLatin1String("data"))
            return url;
        if (url.isLocalFile()) {
            const QString p = QDir::cleanPath(url.toLocalFile());
            // Qt's own QML modules live under its install's qml/ folder.
            if (p.startsWith(m_dir) || p.startsWith(m_qtQml)) return url;
            return QUrl();
        }
        return type == UrlString ? QUrl() : url;
    }
    QString m_dir;
    QString m_qtQml = QDir::cleanPath(QLibraryInfo::path(QLibraryInfo::QmlImportsPath)) + QLatin1Char('/');
};

} // namespace

namespace oynix {

rust::String load_panel(QObject *hostObj, rust::Str qmlPath, rust::Str extDir, QObject *api) {
    auto *host = qobject_cast<QQuickItem *>(hostObj);
    if (!host) return rust::String("no host item");
    const QString path = QString::fromUtf8(qmlPath.data(), qmlPath.size());
    const QString dir = QString::fromUtf8(extDir.data(), extDir.size());

    auto *engine = new QQmlEngine();
    engine->setNetworkAccessManagerFactory(new DenyAllNamFactory);
    engine->addUrlInterceptor(new PanelUrlInterceptor(dir));
    engine->setOfflineStoragePath(QString());
    engine->setBaseUrl(QUrl::fromLocalFile(dir + QLatin1Char('/')));
    engine->rootContext()->setContextProperty(QStringLiteral("extension"), api);

    QFile f(path);
    if (!f.open(QIODevice::ReadOnly)) { delete engine; return rust::String("cannot read graphics.qt"); }
    QQmlComponent component(engine);
    // graphics.qt isn't a .qml name, so hand the source over with a .qml URL.
    component.setData(f.readAll(), QUrl::fromLocalFile(dir + QStringLiteral("/graphics.qml")));
    if (component.isError()) {
        const QString err = component.errorString().trimmed();
        delete engine;
        return rust::String(err.toStdString());
    }
    QObject *obj = component.create();
    auto *item = qobject_cast<QQuickItem *>(obj);
    if (!item) {
        delete obj;
        const QString err = component.isError() ? component.errorString().trimmed() : QStringLiteral("graphics.qt must have an Item as its root");
        delete engine;
        return rust::String(err.toStdString());
    }
    item->setParent(host);
    item->setParentItem(host);
    // The panel's engine lives exactly as long as the panel.
    QObject::connect(item, &QObject::destroyed, engine, &QObject::deleteLater);
    host->setProperty("implicitWidth", item->implicitWidth() > 0 ? item->implicitWidth() : item->width());
    host->setProperty("implicitHeight", item->implicitHeight() > 0 ? item->implicitHeight() : item->height());
    return rust::String();
}

void unload_panel(QObject *hostObj) {
    auto *host = qobject_cast<QQuickItem *>(hostObj);
    if (!host) return;
    const auto items = host->childItems();
    for (QQuickItem *child : items) {
        child->setParentItem(nullptr);
        child->deleteLater();
    }
}

} // namespace oynix
