// Test-only adapter to unmodified, pinned KCachegrind libcore.
#include "tracedata.h"
#include "loader.h"
#include "logger.h"
#include "globalconfig.h"
#include <QCoreApplication>
#include <QFile>
#include <iostream>

class Diagnostics final : public Logger {
public:
    bool failed = false;
    void loadWarning(int line, const QString& msg) override {
        std::cerr << "warning:" << line << ": " << msg.toStdString() << '\n';
    }
    void loadError(int line, const QString& msg) override {
        failed = true;
        std::cerr << "error:" << line << ": " << msg.toStdString() << '\n';
    }
};

static std::string hex(const QString& s) {
    return (s == "???" ? QByteArray() : s.toUtf8().toHex()).toStdString();
}

static std::string identity(TraceFunction* f) {
    return hex(f->object() ? f->object()->name() : QString()) + "\t" +
        hex(f->file() ? f->file()->name() : QString()) + "\t" + hex(f->name());
}

static void costs(ProfileCostArray* c, EventTypeSet* events) {
    for (int i = 0; i < events->realCount(); ++i) {
        if (i) std::cout << ',';
        std::cout << static_cast<uint64>(c->subCost(events->realType(i)));
    }
    std::cout << '\n';
}

int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    if (argc != 2) { std::cerr << "usage: kcachegrind-export PROFILE\n"; return 2; }
    Loader::initLoaders();
    GlobalConfig::setShowCycles(false);
    Diagnostics logger;
    TraceData data(&logger);
    QFile input(QString::fromLocal8Bit(argv[1]));
    // TraceData::load opens the QIODevice itself; pre-opening makes it fail.
    const int loaded = data.load(&input, input.fileName());
    if (logger.failed || loaded <= 0 || data.parts().isEmpty()) return 1;
    auto* events = data.eventTypes();
    int index = 0;
    for (auto* part : data.parts()) {
        data.activateParts(TracePartList{part});
        data.invalidateDynamicCost();
        std::cout << "P\t" << index << '\t';
        for (int i = 0; i < events->realCount(); ++i) {
            if (i) std::cout << ',';
            std::cout << hex(events->realType(i)->name());
        }
        std::cout << "\nT\t" << index << '\t';
        costs(part, events);
        for (auto& value : data.functionMap()) {
            auto* f = &value;
            const auto key = identity(f);
            std::cout << "F\t" << index << '\t' << key << '\t'; costs(f, events);
            std::cout << "I\t" << index << '\t' << key << '\t'; costs(f->inclusive(), events);
            for (auto* edge : f->callings(true)) {
                std::cout << "E\t" << index << '\t' << key << '\t' << identity(edge->called(true))
                          << '\t' << static_cast<uint64>(edge->callCount()) << '\t';
                costs(edge, events);
            }
            for (auto* source : f->sourceFiles()) {
                for (auto& lineValue : *source->lineMap()) {
                    auto* line = &lineValue;
                    if (!line->lineno()) continue;
                    std::cout << "L\t" << index << '\t' << key << '\t' << hex(source->file()->name())
                              << '\t' << line->lineno() << '\t';
                    costs(line, events);
                }
            }
        }
        ++index;
    }
    return logger.failed ? 1 : 0;
}
