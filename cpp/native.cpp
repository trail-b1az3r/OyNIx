#include "oynix/cpp/native.h"
#include "oynix/src/bridge/native.cxx.h"

#include <QtCore/QByteArray>
#include <QtCore/QCoreApplication>
#include <QtCore/QFile>
#include <QtCore/QString>
#include <QtCore/QUrl>
#include <QtCore/QtGlobal>
#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>
#include <QtGui/QSurfaceFormat>
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

rust::String webchannel_js() {
    QFile f(QStringLiteral(":/qtwebchannel/qwebchannel.js"));
    if (!f.open(QIODevice::ReadOnly)) return rust::String();
    const QByteArray b = f.readAll();
    return rust::String(b.constData(), static_cast<size_t>(b.size()));
}

rust::String qt_runtime_version() { return rust::String(qVersion()); }

} // namespace oynix
