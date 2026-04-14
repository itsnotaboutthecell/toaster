#include "MainWindow.h"
#include "WaveformView.h"

#include <QAbstractItemView>
#include <QAction>
#include <QApplication>
#include <QAudioOutput>
#include <QCloseEvent>
#include <QCoreApplication>
#include <QCryptographicHash>
#include <QDateTime>
#include <QDockWidget>
#include <QDir>
#include <QElapsedTimer>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QFormLayout>
#include <QFrame>
#include <QHeaderView>
#include <QHBoxLayout>
#include <QItemSelectionModel>
#include <QImage>
#include <QKeySequence>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QListWidgetItem>
#include <QLoggingCategory>
#include <QMediaPlayer>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QProcess>
#include <QPushButton>
#include <QRegularExpression>
#include <QSettings>
#include <QSignalBlocker>
#include <QSlider>
#include <QStandardPaths>
#include <QStatusBar>
#include <QTableWidget>
#include <QTableWidgetItem>
#include <QTemporaryDir>
#include <QThread>
#include <QTextStream>
#include <QToolButton>
#include <QUrl>
#include <QVBoxLayout>
#include <QVideoWidget>
#include <QWidget>

#include <algorithm>
#include <array>
#include <limits>

#ifdef Q_OS_WIN
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#include <shlobj.h>
#include <shobjidl.h>
#endif

namespace {

QDockWidget *createDock(QMainWindow *window, const QString &title, const QString &objectName,
                        QWidget *widget)
{
  auto *dock = new QDockWidget(title, window);
  dock->setObjectName(objectName);
  dock->setAllowedAreas(Qt::AllDockWidgetAreas);
  dock->setFeatures(QDockWidget::DockWidgetClosable | QDockWidget::DockWidgetMovable |
                    QDockWidget::DockWidgetFloatable);
  dock->setWidget(widget);
  return dock;
}

QString formatMicros(qint64 value)
{
  qint64 totalSeconds = value / 1000000;
  qint64 minutes = totalSeconds / 60;
  qint64 seconds = totalSeconds % 60;
  qint64 milliseconds = (value % 1000000) / 1000;
  return QString("%1:%2.%3")
    .arg(minutes, 2, 10, QLatin1Char('0'))
    .arg(seconds, 2, 10, QLatin1Char('0'))
    .arg(milliseconds, 3, 10, QLatin1Char('0'));
}

QString formatSecondsCell(qint64 value)
{
  return QString::number(static_cast<double>(value) / 1000000.0, 'f', 3);
}

QString wordStateLabel(const toaster_word_t &word)
{
  if (word.deleted && word.silenced)
    return "deleted + silenced";
  if (word.deleted)
    return "deleted";
  if (word.silenced)
    return "silenced";
  return "active";
}

QString suggestionKindLabel(toaster_suggestion_kind_t kind)
{
  switch (kind) {
  case TOASTER_SUGGESTION_DELETE_FILLER:
    return "Delete filler";
  case TOASTER_SUGGESTION_SILENCE_FILLER:
    return "Silence filler";
  case TOASTER_SUGGESTION_SHORTEN_PAUSE:
    return "Shorten pause";
  }

  return "Suggestion";
}

QColor transcriptRowBackground(const toaster_word_t &word, bool active)
{
  if (active && word.silenced)
    return QColor(180, 205, 255, 140);
  if (active)
    return QColor(255, 224, 110, 110);
  if (word.silenced)
    return QColor(30, 90, 160, 60);
  return QColor();
}

QStringList splitTranscriptWords(const QString &text)
{
  return text.split(QRegularExpression("\\s+"), Qt::SkipEmptyParts);
}

QString firstUsableLocation(QStandardPaths::StandardLocation primary,
                            QStandardPaths::StandardLocation fallback)
{
  QString path = QStandardPaths::writableLocation(primary);

  if (!path.isEmpty())
    return path;

  path = QStandardPaths::writableLocation(fallback);
  if (!path.isEmpty())
    return path;

  return QDir::homePath();
}

QString defaultMediaDialogPath()
{
  return firstUsableLocation(QStandardPaths::MoviesLocation, QStandardPaths::DownloadLocation);
}

QString defaultDocumentDialogPath()
{
  return firstUsableLocation(QStandardPaths::DocumentsLocation, QStandardPaths::DownloadLocation);
}

QString suggestedProjectPath(const toaster_project_t *project, const QString &currentProjectPath)
{
  if (!currentProjectPath.isEmpty())
    return currentProjectPath;

  if (project) {
    QString mediaPath = QString::fromUtf8(toaster_project_get_media_path(project));
    if (!mediaPath.isEmpty()) {
      QFileInfo info(mediaPath);
      return info.dir().filePath(info.completeBaseName() + ".toaster");
    }
  }

  return QDir(defaultDocumentDialogPath()).filePath("untitled.toaster");
}

QString suggestedTranscriptPath(const toaster_project_t *project)
{
  if (project) {
    QString mediaPath = QString::fromUtf8(toaster_project_get_media_path(project));
    if (!mediaPath.isEmpty())
      return QFileInfo(mediaPath).absolutePath();
  }

  return defaultDocumentDialogPath();
}

QString suggestedExportPath(const toaster_project_t *project)
{
  if (project) {
    QString mediaPath = QString::fromUtf8(toaster_project_get_media_path(project));
    if (!mediaPath.isEmpty()) {
      QFileInfo info(mediaPath);
      return info.dir().filePath(info.completeBaseName() + "-edited.mp4");
    }
  }

  return QDir(defaultDocumentDialogPath()).filePath("edited.mp4");
}

QString suggestedTextExportBasePath(const toaster_project_t *project, const QString &currentProjectPath,
                                    const QString &fallbackStem)
{
  if (project) {
    QString mediaPath = QString::fromUtf8(toaster_project_get_media_path(project));
    if (!mediaPath.isEmpty()) {
      QFileInfo info(mediaPath);
      return info.dir().filePath(info.completeBaseName());
    }
  }

  if (!currentProjectPath.isEmpty()) {
    QFileInfo info(currentProjectPath);
    return info.dir().filePath(info.completeBaseName());
  }

  return QDir(defaultDocumentDialogPath()).filePath(fallbackStem);
}

QString suggestedCaptionPath(const toaster_project_t *project, const QString &currentProjectPath)
{
  return suggestedTextExportBasePath(project, currentProjectPath, "captions") + ".srt";
}

QString suggestedScriptPath(const toaster_project_t *project, const QString &currentProjectPath)
{
  return suggestedTextExportBasePath(project, currentProjectPath, "script") + ".txt";
}

toaster_caption_format_t captionFormatForPath(const QString &path)
{
  return QFileInfo(path).suffix().compare("vtt", Qt::CaseInsensitive) == 0
           ? TOASTER_CAPTION_FORMAT_VTT
           : TOASTER_CAPTION_FORMAT_SRT;
}

bool isSpecialWhisperToken(const QString &token)
{
  return token.startsWith('[') && token.endsWith(']');
}

bool shouldAttachWhisperToken(const QString &token)
{
  if (token.isEmpty())
    return false;

  const QChar first = token.front();
  return first.isPunct() || first == '\'' || first == '-';
}

struct ScopedFlag {
  bool &flag;

  explicit ScopedFlag(bool &value) : flag(value) { flag = true; }
  ~ScopedFlag() { flag = false; }
};

#ifdef Q_OS_WIN
struct Win32FilterSpec {
  std::vector<std::wstring> names;
  std::vector<std::wstring> patterns;
  std::vector<COMDLG_FILTERSPEC> specs;

  explicit Win32FilterSpec(const QString &qtFilter)
  {
    QStringList entries = qtFilter.split(";;", Qt::SkipEmptyParts);
    if (entries.isEmpty())
      entries << "All Files (*.*)";

    names.reserve(entries.size());
    patterns.reserve(entries.size());
    specs.reserve(entries.size());

    for (const QString &entry : entries) {
      QString label = entry.trimmed();
      QString pattern = "*.*";
      int openParen = entry.lastIndexOf('(');
      int closeParen = entry.lastIndexOf(')');

      if (openParen >= 0 && closeParen > openParen) {
        pattern = entry.mid(openParen + 1, closeParen - openParen - 1).trimmed();
        pattern.replace(' ', ';');
      }

      names.push_back(label.toStdWString());
      patterns.push_back(pattern.toStdWString());
      specs.push_back({names.back().c_str(), patterns.back().c_str()});
    }
  }
};

QString getNativeOpenFileName(QWidget *parent, const QString &title, const QString &initialPath,
                              const QString &filter)
{
  IFileOpenDialog *dialog = nullptr;
  HRESULT hr = CoCreateInstance(CLSID_FileOpenDialog, nullptr, CLSCTX_INPROC_SERVER,
                                IID_PPV_ARGS(&dialog));
  if (FAILED(hr))
    return QFileDialog::getOpenFileName(parent, title, initialPath, filter);

  Win32FilterSpec filterSpec(filter);
  dialog->SetFileTypes(static_cast<UINT>(filterSpec.specs.size()), filterSpec.specs.data());
  dialog->SetTitle(title.toStdWString().c_str());
  dialog->SetOptions(FOS_FILEMUSTEXIST | FOS_PATHMUSTEXIST | FOS_FORCEFILESYSTEM);

  QFileInfo info(initialPath);
  if (!initialPath.isEmpty()) {
    QString dir = info.isDir() ? info.absoluteFilePath() : info.absolutePath();
    IShellItem *folder = nullptr;
    hr = SHCreateItemFromParsingName(QDir::toNativeSeparators(dir).toStdWString().c_str(),
                                     nullptr, IID_PPV_ARGS(&folder));
    if (SUCCEEDED(hr)) {
      dialog->SetFolder(folder);
      folder->Release();
    }
    if (!info.isDir() && !info.fileName().isEmpty())
      dialog->SetFileName(info.fileName().toStdWString().c_str());
  }

  HWND owner = parent ? reinterpret_cast<HWND>(parent->window()->winId()) : nullptr;
  hr = dialog->Show(owner);
  if (FAILED(hr)) {
    dialog->Release();
    return QString();
  }

  IShellItem *result = nullptr;
  hr = dialog->GetResult(&result);
  dialog->Release();
  if (FAILED(hr) || !result)
    return QString();

  PWSTR path = nullptr;
  hr = result->GetDisplayName(SIGDN_FILESYSPATH, &path);
  result->Release();
  if (FAILED(hr) || !path)
    return QString();

  QString selected = QDir::fromNativeSeparators(QString::fromWCharArray(path));
  CoTaskMemFree(path);
  return selected;
}

QString getNativeSaveFileName(QWidget *parent, const QString &title, const QString &initialPath,
                              const QString &filter, const QString &defaultSuffix)
{
  IFileSaveDialog *dialog = nullptr;
  HRESULT hr = CoCreateInstance(CLSID_FileSaveDialog, nullptr, CLSCTX_INPROC_SERVER,
                                IID_PPV_ARGS(&dialog));
  if (FAILED(hr)) {
    QString selected = QFileDialog::getSaveFileName(parent, title, initialPath, filter);
    if (!selected.isEmpty() && !defaultSuffix.isEmpty() &&
        !selected.endsWith("." + defaultSuffix, Qt::CaseInsensitive))
      selected += "." + defaultSuffix;
    return selected;
  }

  Win32FilterSpec filterSpec(filter);
  dialog->SetFileTypes(static_cast<UINT>(filterSpec.specs.size()), filterSpec.specs.data());
  dialog->SetTitle(title.toStdWString().c_str());
  dialog->SetOptions(FOS_OVERWRITEPROMPT | FOS_PATHMUSTEXIST | FOS_FORCEFILESYSTEM);

  if (!defaultSuffix.isEmpty())
    dialog->SetDefaultExtension(defaultSuffix.toStdWString().c_str());

  QFileInfo info(initialPath);
  if (!initialPath.isEmpty()) {
    QString dir = info.isDir() ? info.absoluteFilePath() : info.absolutePath();
    IShellItem *folder = nullptr;
    hr = SHCreateItemFromParsingName(QDir::toNativeSeparators(dir).toStdWString().c_str(),
                                     nullptr, IID_PPV_ARGS(&folder));
    if (SUCCEEDED(hr)) {
      dialog->SetFolder(folder);
      folder->Release();
    }
    if (!info.isDir() && !info.fileName().isEmpty())
      dialog->SetFileName(info.fileName().toStdWString().c_str());
  }

  HWND owner = parent ? reinterpret_cast<HWND>(parent->window()->winId()) : nullptr;
  hr = dialog->Show(owner);
  if (FAILED(hr)) {
    dialog->Release();
    return QString();
  }

  IShellItem *result = nullptr;
  hr = dialog->GetResult(&result);
  dialog->Release();
  if (FAILED(hr) || !result)
    return QString();

  PWSTR path = nullptr;
  hr = result->GetDisplayName(SIGDN_FILESYSPATH, &path);
  result->Release();
  if (FAILED(hr) || !path)
    return QString();

  QString selected = QDir::fromNativeSeparators(QString::fromWCharArray(path));
  CoTaskMemFree(path);
  return selected;
}
#else
QString getNativeOpenFileName(QWidget *parent, const QString &title, const QString &initialPath,
                              const QString &filter)
{
  return QFileDialog::getOpenFileName(parent, title, initialPath, filter);
}

QString getNativeSaveFileName(QWidget *parent, const QString &title, const QString &initialPath,
                              const QString &filter, const QString &defaultSuffix)
{
  QString selected = QFileDialog::getSaveFileName(parent, title, initialPath, filter);

  if (!selected.isEmpty() && !defaultSuffix.isEmpty() &&
      !selected.endsWith("." + defaultSuffix, Qt::CaseInsensitive)) {
    selected += "." + defaultSuffix;
  }

  return selected;
}
#endif

}  // namespace

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent)
{
  setWindowTitle(QString("Toaster %1").arg(QString::fromUtf8(toaster_get_version())));
  resize(1440, 900);
  setDockOptions(QMainWindow::AnimatedDocks | QMainWindow::AllowNestedDocks |
                 QMainWindow::AllowTabbedDocks | QMainWindow::GroupedDragging);

  m_suggestions = toaster_suggestion_list_create();

  createCentralWorkspace();
  createDocks();
  createMenus();
  wirePlayer();
  restoreWindowState();
  newProject();

  statusBar()->showMessage("Toaster ready");
  appendLogLine("Toaster shell booted.");
}

MainWindow::~MainWindow()
{
  toaster_suggestion_list_destroy(m_suggestions);
  toaster_project_destroy(m_project);
}

void MainWindow::closeEvent(QCloseEvent *event)
{
  saveWindowState();
  QMainWindow::closeEvent(event);
}

void MainWindow::createCentralWorkspace()
{
  auto *frame = new QFrame(this);
  auto *layout = new QVBoxLayout(frame);
  auto *transportLayout = new QHBoxLayout();

  frame->setFrameShape(QFrame::StyledPanel);
  layout->setContentsMargins(16, 16, 16, 16);
  layout->setSpacing(12);

  m_player = new QMediaPlayer(this);
  m_audioOutput = new QAudioOutput(this);
  m_audioOutput->setVolume(1.0);
  m_player->setAudioOutput(m_audioOutput);

  m_videoWidget = new QVideoWidget(frame);
  m_videoWidget->setMinimumSize(960, 540);
  m_player->setVideoOutput(m_videoWidget);

  m_playButton = new QPushButton("Play", frame);
  m_positionSlider = new QSlider(Qt::Horizontal, frame);
  m_positionSlider->setRange(0, 0);

  transportLayout->addWidget(m_playButton);
  transportLayout->addWidget(m_positionSlider, 1);

  layout->addWidget(m_videoWidget, 1);
  layout->addLayout(transportLayout);
  setCentralWidget(frame);
}

void MainWindow::createDocks()
{
  auto *transcriptWidget = new QWidget(this);
  auto *transcriptLayout = new QVBoxLayout(transcriptWidget);
  auto *transcriptWorkflowLayout = new QHBoxLayout();
  auto *transcriptSearchLayout = new QHBoxLayout();

  transcriptLayout->setContentsMargins(8, 8, 8, 8);
  transcriptLayout->setSpacing(6);
  transcriptWorkflowLayout->setContentsMargins(0, 0, 0, 0);
  transcriptSearchLayout->setContentsMargins(0, 0, 0, 0);

  m_transcriptTable = new QTableWidget(transcriptWidget);
  m_transcriptTable->setColumnCount(4);
  m_transcriptTable->setHorizontalHeaderLabels({"Word", "Start (s)", "End (s)", "State"});
  m_transcriptTable->setSelectionBehavior(QAbstractItemView::SelectRows);
  m_transcriptTable->setSelectionMode(QAbstractItemView::ExtendedSelection);
  m_transcriptTable->horizontalHeader()->setStretchLastSection(true);
  m_transcriptTable->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
  m_transcriptTable->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
  m_transcriptTable->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents);
  m_transcriptTable->horizontalHeader()->setSectionResizeMode(3, QHeaderView::ResizeToContents);

  m_transcriptTranscribeButton = new QToolButton(transcriptWidget);
  m_transcriptTranscribeButton->setToolButtonStyle(Qt::ToolButtonTextOnly);
  m_transcriptImportButton = new QToolButton(transcriptWidget);
  m_transcriptImportButton->setToolButtonStyle(Qt::ToolButtonTextOnly);
  transcriptWorkflowLayout->addWidget(m_transcriptTranscribeButton);
  transcriptWorkflowLayout->addWidget(m_transcriptImportButton);
  transcriptWorkflowLayout->addStretch();

  m_transcriptSearchEdit = new QLineEdit(transcriptWidget);
  m_transcriptSearchEdit->setClearButtonEnabled(true);
  m_transcriptSearchEdit->setPlaceholderText("Search transcript (Ctrl+F)");
  m_transcriptSearchPreviousButton = new QToolButton(transcriptWidget);
  m_transcriptSearchPreviousButton->setToolButtonStyle(Qt::ToolButtonTextOnly);
  m_transcriptSearchNextButton = new QToolButton(transcriptWidget);
  m_transcriptSearchNextButton->setToolButtonStyle(Qt::ToolButtonTextOnly);
  m_transcriptSearchStatusLabel = new QLabel("No transcript", transcriptWidget);
  m_transcriptSearchStatusLabel->setAlignment(Qt::AlignRight | Qt::AlignVCenter);

  transcriptSearchLayout->addWidget(m_transcriptSearchEdit, 1);
  transcriptSearchLayout->addWidget(m_transcriptSearchPreviousButton);
  transcriptSearchLayout->addWidget(m_transcriptSearchNextButton);
  transcriptSearchLayout->addWidget(m_transcriptSearchStatusLabel);

  transcriptLayout->addLayout(transcriptWorkflowLayout);
  transcriptLayout->addLayout(transcriptSearchLayout);
  transcriptLayout->addWidget(m_transcriptTable, 1);
  m_transcriptDock = createDock(this, "Transcript", "transcriptDock", transcriptWidget);

  m_waveformView = new WaveformView(this);
  m_waveformDock = createDock(this, "Waveform / Timeline", "waveformDock", m_waveformView);

  auto *suggestedEditsWidget = new QWidget(this);
  auto *suggestedEditsLayout = new QVBoxLayout(suggestedEditsWidget);
  auto *analyzeButton = new QPushButton("Analyze Transcript", suggestedEditsWidget);
  auto *applySelectedButton = new QPushButton("Apply Selected", suggestedEditsWidget);
  auto *applyAllButton = new QPushButton("Apply All", suggestedEditsWidget);
  m_suggestionList = new QListWidget(suggestedEditsWidget);
  suggestedEditsLayout->addWidget(analyzeButton);
  suggestedEditsLayout->addWidget(applySelectedButton);
  suggestedEditsLayout->addWidget(applyAllButton);
  suggestedEditsLayout->addWidget(m_suggestionList, 1);
  m_suggestedEditsDock =
    createDock(this, "Suggested Edits", "suggestedEditsDock", suggestedEditsWidget);

  auto *inspectorWidget = new QWidget(this);
  auto *inspectorLayout = new QFormLayout(inspectorWidget);
  m_projectLabel = new QLabel("Untitled", inspectorWidget);
  m_mediaLabel = new QLabel("No media loaded", inspectorWidget);
  m_mediaLabel->setWordWrap(true);
  m_durationLabel = new QLabel("0:00.000", inspectorWidget);
  m_languageLabel = new QLabel("en", inspectorWidget);
  m_statsLabel = new QLabel("Words: 0\nDeleted: 0\nSilenced: 0\nCuts: 0\nSuggestions: 0",
                            inspectorWidget);
  m_statsLabel->setTextFormat(Qt::PlainText);
  inspectorLayout->addRow("Project", m_projectLabel);
  inspectorLayout->addRow("Media", m_mediaLabel);
  inspectorLayout->addRow("Duration", m_durationLabel);
  inspectorLayout->addRow("Language", m_languageLabel);
  inspectorLayout->addRow("Stats", m_statsLabel);
  m_inspectorDock = createDock(this, "Inspector", "inspectorDock", inspectorWidget);

  auto *exportWidget = new QWidget(this);
  auto *exportLayout = new QVBoxLayout(exportWidget);
  auto *exportHelp = new QLabel(
    "Export edited media, sidecar captions, or a clean script from the current transcript state.",
    exportWidget);
  exportHelp->setWordWrap(true);
  m_exportMediaButton = new QToolButton(exportWidget);
  m_exportMediaButton->setToolButtonStyle(Qt::ToolButtonTextOnly);
  m_exportCaptionsButton = new QToolButton(exportWidget);
  m_exportCaptionsButton->setToolButtonStyle(Qt::ToolButtonTextOnly);
  m_exportScriptButton = new QToolButton(exportWidget);
  m_exportScriptButton->setToolButtonStyle(Qt::ToolButtonTextOnly);
  exportLayout->addWidget(exportHelp);
  exportLayout->addWidget(m_exportMediaButton);
  exportLayout->addWidget(m_exportCaptionsButton);
  exportLayout->addWidget(m_exportScriptButton);
  exportLayout->addStretch();
  m_exportDock = createDock(this, "Export", "exportDock", exportWidget);

  m_logView = new QPlainTextEdit(this);
  m_logView->setReadOnly(true);
  m_logsDock = createDock(this, "Logs", "logsDock", m_logView);

  m_docks = {m_transcriptDock, m_waveformDock, m_suggestedEditsDock,
             m_inspectorDock, m_exportDock,   m_logsDock};

  addDockWidget(Qt::RightDockWidgetArea, m_transcriptDock);
  addDockWidget(Qt::BottomDockWidgetArea, m_waveformDock);
  addDockWidget(Qt::RightDockWidgetArea, m_suggestedEditsDock);
  addDockWidget(Qt::RightDockWidgetArea, m_inspectorDock);
  addDockWidget(Qt::RightDockWidgetArea, m_exportDock);
  addDockWidget(Qt::BottomDockWidgetArea, m_logsDock);

  tabifyDockWidget(m_transcriptDock, m_suggestedEditsDock);
  tabifyDockWidget(m_transcriptDock, m_inspectorDock);
  tabifyDockWidget(m_transcriptDock, m_exportDock);
  tabifyDockWidget(m_waveformDock, m_logsDock);

  m_transcriptDock->raise();
  m_waveformDock->raise();

  connect(analyzeButton, &QPushButton::clicked, this, &MainWindow::analyzeCleanup);
  connect(applySelectedButton, &QPushButton::clicked, this, &MainWindow::applySelectedSuggestion);
  connect(applyAllButton, &QPushButton::clicked, this, &MainWindow::applyAllSuggestions);
  connect(m_transcriptTable, &QTableWidget::cellChanged, this, &MainWindow::onTranscriptCellChanged);
  connect(m_transcriptTable->selectionModel(), &QItemSelectionModel::selectionChanged, this,
          [this]() { onTranscriptSelectionChanged(); });
  connect(m_transcriptSearchEdit, &QLineEdit::textChanged, this, &MainWindow::onTranscriptSearchChanged);
  connect(m_transcriptSearchEdit, &QLineEdit::returnPressed, this, &MainWindow::findNextTranscriptMatch);
  connect(m_suggestionList, &QListWidget::currentItemChanged, this,
          [this]() { onSuggestionSelectionChanged(); });
  connect(m_suggestionList, &QListWidget::itemDoubleClicked, this,
          [this]() { onSuggestionActivated(); });
  connect(m_waveformView, &WaveformView::seekRequested, this, &MainWindow::onWaveformSeekRequested);
}

void MainWindow::createMenus()
{
  auto *fileMenu = menuBar()->addMenu("&File");
  m_importTranscriptAction = new QAction("Import Transcript Text...", this);
  connect(m_importTranscriptAction, &QAction::triggered, this, &MainWindow::importTranscript);
  fileMenu->addAction("New Project", this, &MainWindow::newProject);
  fileMenu->addAction("Open Media", this, &MainWindow::openMediaDialog);
  fileMenu->addAction("Open Project", this, &MainWindow::openProjectDialog);
  m_saveProjectAction = fileMenu->addAction("Save Project", this, &MainWindow::saveProject);
  fileMenu->addAction("Save Project As", this, &MainWindow::saveProjectAs);
  fileMenu->addSeparator();
  fileMenu->addAction(m_importTranscriptAction);
  m_exportMediaAction = new QAction("Export Media", this);
  connect(m_exportMediaAction, &QAction::triggered, this, &MainWindow::exportMedia);
  m_exportCaptionsAction = new QAction("Export Captions...", this);
  connect(m_exportCaptionsAction, &QAction::triggered, this, &MainWindow::exportCaptions);
  m_exportScriptAction = new QAction("Export Script...", this);
  connect(m_exportScriptAction, &QAction::triggered, this, &MainWindow::exportScript);
  fileMenu->addAction(m_exportMediaAction);
  fileMenu->addAction(m_exportCaptionsAction);
  fileMenu->addAction(m_exportScriptAction);
  fileMenu->addSeparator();
  fileMenu->addAction("Exit", this, &QWidget::close);

  auto *editMenu = menuBar()->addMenu("&Edit");
  m_focusTranscriptSearchAction = new QAction("Find Transcript", this);
  m_focusTranscriptSearchAction->setShortcut(QKeySequence::Find);
  connect(m_focusTranscriptSearchAction, &QAction::triggered, this, &MainWindow::focusTranscriptSearch);
  m_findNextAction = new QAction("Find Next", this);
  m_findNextAction->setShortcut(QKeySequence::FindNext);
  connect(m_findNextAction, &QAction::triggered, this, &MainWindow::findNextTranscriptMatch);
  m_findPreviousAction = new QAction("Find Previous", this);
  m_findPreviousAction->setShortcut(QKeySequence::FindPrevious);
  connect(m_findPreviousAction, &QAction::triggered, this, &MainWindow::findPreviousTranscriptMatch);
  addAction(m_focusTranscriptSearchAction);
  addAction(m_findNextAction);
  addAction(m_findPreviousAction);
  editMenu->addAction(m_focusTranscriptSearchAction);
  editMenu->addAction(m_findNextAction);
  editMenu->addAction(m_findPreviousAction);
  editMenu->addSeparator();
  editMenu->addAction("Delete Selection", this, &MainWindow::deleteSelection);
  editMenu->addAction("Silence Selection", this, &MainWindow::silenceSelection);
  editMenu->addAction("Unsilence Selection", this, &MainWindow::unsilenceSelection);
  editMenu->addAction("Restore Selection", this, &MainWindow::restoreSelection);

  auto *viewMenu = menuBar()->addMenu("&View");
  auto *statusBarAction = viewMenu->addAction("Status Bar");
  statusBarAction->setCheckable(true);
  statusBarAction->setChecked(true);
  connect(statusBarAction, &QAction::toggled, this, [this](bool visible) {
    statusBar()->setVisible(visible);
    appendLogLine(QString("Status bar %1.").arg(visible ? "shown" : "hidden"));
  });
  m_alwaysOnTopAction = viewMenu->addAction("Always on Top");
  m_alwaysOnTopAction->setCheckable(true);
  connect(m_alwaysOnTopAction, &QAction::toggled, this, [this](bool enabled) {
    setWindowFlag(Qt::WindowStaysOnTopHint, enabled);
    show();
    appendLogLine(QString("Always on top %1.").arg(enabled ? "enabled" : "disabled"));
  });

  auto *docksMenu = menuBar()->addMenu("&Docks");
  for (QDockWidget *dock : m_docks)
    docksMenu->addAction(dock->toggleViewAction());
  docksMenu->addSeparator();
  m_lockDocksAction = docksMenu->addAction("Lock Docks");
  m_lockDocksAction->setCheckable(true);
  connect(m_lockDocksAction, &QAction::toggled, this, &MainWindow::setDocksLocked);
  docksMenu->addAction("Reset Docks", this, &MainWindow::resetDocks);

  auto *projectMenu = menuBar()->addMenu("&Project");
  m_transcribeAction = new QAction("Transcribe Media", this);
  connect(m_transcribeAction, &QAction::triggered, this, &MainWindow::transcribeMedia);
  projectMenu->addAction(m_transcribeAction);
  projectMenu->addAction(m_importTranscriptAction);

  auto *profilesMenu = menuBar()->addMenu("P&rofiles");
  profilesMenu->addAction("Default Cleanup Profile", this,
                          [this]() { appendLogLine("Default cleanup profile active."); });
  profilesMenu->addAction("Default Export Profile", this,
                          [this]() { appendLogLine("Default export profile active."); });

  auto *toolsMenu = menuBar()->addMenu("&Tools");
  toolsMenu->addAction(m_transcribeAction);
  m_analyzeAction = toolsMenu->addAction("Suggested Edits", this, &MainWindow::analyzeCleanup);
  toolsMenu->addAction("Apply Selected Suggestion", this, &MainWindow::applySelectedSuggestion);
  toolsMenu->addAction("Apply All Suggestions", this, &MainWindow::applyAllSuggestions);

  if (m_transcriptTranscribeButton)
    m_transcriptTranscribeButton->setDefaultAction(m_transcribeAction);
  if (m_transcriptImportButton)
    m_transcriptImportButton->setDefaultAction(m_importTranscriptAction);
  if (m_transcriptSearchPreviousButton)
    m_transcriptSearchPreviousButton->setDefaultAction(m_findPreviousAction);
  if (m_transcriptSearchNextButton)
    m_transcriptSearchNextButton->setDefaultAction(m_findNextAction);
  if (m_exportMediaButton)
    m_exportMediaButton->setDefaultAction(m_exportMediaAction);
  if (m_exportCaptionsButton)
    m_exportCaptionsButton->setDefaultAction(m_exportCaptionsAction);
  if (m_exportScriptButton)
    m_exportScriptButton->setDefaultAction(m_exportScriptAction);

  auto *helpMenu = menuBar()->addMenu("&Help");
  helpMenu->addAction("About", this, [this]() {
    QMessageBox::information(
      this, "About Toaster",
      "Native transcript-first editor bootstrap.\n\n"
      "Current implementation covers project I/O, real waveform display, local whisper.cpp "
      "transcription, transcript search/navigation, plain-text transcript import, FFmpeg "
      "media export, sidecar caption/script export, and deterministic cleanup analysis.");
  });
}

void MainWindow::wirePlayer()
{
  connect(m_playButton, &QPushButton::clicked, this, [this]() {
    if (m_player->playbackState() == QMediaPlayer::PlayingState)
      m_player->pause();
    else
      m_player->play();
  });
  connect(m_player, &QMediaPlayer::playbackStateChanged, this, [this](QMediaPlayer::PlaybackState state) {
    m_playButton->setText(state == QMediaPlayer::PlayingState ? "Pause" : "Play");
  });
  connect(m_player, &QMediaPlayer::durationChanged, this, &MainWindow::onDurationChanged);
  connect(m_player, &QMediaPlayer::positionChanged, this, &MainWindow::onPositionChanged);
  connect(m_positionSlider, &QSlider::sliderMoved, this, &MainWindow::onSliderMoved);
  connect(m_player, &QMediaPlayer::errorOccurred, this,
          [this](QMediaPlayer::Error, const QString &message) {
            if (!message.isEmpty())
              appendLogLine(QString("Media error: %1").arg(message));
          });
}

void MainWindow::restoreWindowState()
{
  QSettings settings("Toaster", "Toaster");
  QByteArray geometry = settings.value("mainWindow/geometry").toByteArray();
  QByteArray state = settings.value("mainWindow/state").toByteArray();
  bool docksLocked = settings.value("mainWindow/docksLocked", false).toBool();
  bool alwaysOnTop = settings.value("mainWindow/alwaysOnTop", false).toBool();

  if (!geometry.isEmpty())
    restoreGeometry(geometry);

  if (state.isEmpty() || !restoreState(state))
    resetDocks();

  if (m_lockDocksAction) {
    m_lockDocksAction->blockSignals(true);
    m_lockDocksAction->setChecked(docksLocked);
    m_lockDocksAction->blockSignals(false);
  }
  setDocksLocked(docksLocked);

  if (m_alwaysOnTopAction) {
    m_alwaysOnTopAction->blockSignals(true);
    m_alwaysOnTopAction->setChecked(alwaysOnTop);
    m_alwaysOnTopAction->blockSignals(false);
  }
  setWindowFlag(Qt::WindowStaysOnTopHint, alwaysOnTop);
}

void MainWindow::saveWindowState()
{
  QSettings settings("Toaster", "Toaster");
  settings.setValue("mainWindow/geometry", saveGeometry());
  settings.setValue("mainWindow/state", saveState());
  settings.setValue("mainWindow/docksLocked",
                    m_lockDocksAction ? m_lockDocksAction->isChecked() : false);
  settings.setValue("mainWindow/alwaysOnTop",
                    m_alwaysOnTopAction ? m_alwaysOnTopAction->isChecked() : false);
}

void MainWindow::setDocksLocked(bool locked)
{
  QDockWidget::DockWidgetFeatures features = QDockWidget::DockWidgetClosable;

  if (!locked) {
    features = QDockWidget::DockWidgetClosable | QDockWidget::DockWidgetMovable |
               QDockWidget::DockWidgetFloatable;
  }

  for (QDockWidget *dock : m_docks)
    dock->setFeatures(features);

  appendLogLine(QString("Docks %1.").arg(locked ? "locked" : "unlocked"));
}

void MainWindow::resetDocks()
{
  addDockWidget(Qt::RightDockWidgetArea, m_transcriptDock);
  addDockWidget(Qt::RightDockWidgetArea, m_suggestedEditsDock);
  addDockWidget(Qt::RightDockWidgetArea, m_inspectorDock);
  addDockWidget(Qt::RightDockWidgetArea, m_exportDock);
  addDockWidget(Qt::BottomDockWidgetArea, m_waveformDock);
  addDockWidget(Qt::BottomDockWidgetArea, m_logsDock);

  tabifyDockWidget(m_transcriptDock, m_suggestedEditsDock);
  tabifyDockWidget(m_transcriptDock, m_inspectorDock);
  tabifyDockWidget(m_transcriptDock, m_exportDock);
  tabifyDockWidget(m_waveformDock, m_logsDock);

  for (QDockWidget *dock : m_docks)
    dock->show();

  m_transcriptDock->raise();
  m_waveformDock->raise();
  appendLogLine("Dock layout reset.");
}

void MainWindow::appendLogLine(const QString &line)
{
  if (m_logView)
    m_logView->appendPlainText(line);

  qInfo().noquote() << line;
  statusBar()->showMessage(line, 4000);
}

bool MainWindow::ensureProject()
{
  if (m_project)
    return true;

  m_project = toaster_project_create();
  if (!m_project) {
    appendLogLine("Failed to allocate project.");
    return false;
  }

  toaster_project_set_language(m_project, "en-US");
  return true;
}

void MainWindow::clearProject()
{
  if (m_player) {
    m_player->stop();
    m_player->setSource(QUrl());
  }

  if (m_waveformView)
    m_waveformView->clear();

  toaster_project_destroy(m_project);
  m_project = NULL;
  m_projectPath.clear();
  m_lastTranscriptionError.clear();
  m_mediaDurationUs = 0;
  m_activeTranscriptRow = -1;

  if (m_suggestions)
    toaster_suggestion_list_clear(m_suggestions);

  m_positionSlider->setRange(0, 0);
  clearTranscriptSearch();
}

void MainWindow::newProject()
{
  clearProject();
  if (!ensureProject())
    return;

  rebuildAllViews();
  appendLogLine("New project created.");
}

void MainWindow::openMedia(const QString &path)
{
  QString transcriptionError;
  QString waveformError;
  toaster_transcript_t *transcript;

  if (path.isEmpty())
    return;

  if (!ensureProject())
    return;

  transcript = toaster_project_get_transcript(m_project);
  toaster_transcript_clear(transcript);
  toaster_project_set_media_path(m_project, path.toUtf8().constData());
  toaster_project_set_language(m_project, "en-US");
  toaster_suggestion_list_clear(m_suggestions);
  clearTranscriptSearch();

  m_player->setSource(QUrl::fromLocalFile(path));
  m_player->pause();
  m_mediaDurationUs = 0;
  m_lastTranscriptionError.clear();
  m_positionSlider->setValue(0);
  if (!loadWaveformForMedia(path, &waveformError) && !waveformError.isEmpty())
    appendLogLine(waveformError);
  if (!transcribeCurrentMedia(true, &transcriptionError) && !transcriptionError.isEmpty()) {
    m_lastTranscriptionError = transcriptionError;
    appendLogLine(transcriptionError);
  }

  rebuildAllViews();
  appendLogLine(QString("Opened media: %1").arg(path));
}

void MainWindow::openMediaDialog()
{
  QString path = getNativeOpenFileName(
    this, "Open Media", defaultMediaDialogPath(),
    "Media Files (*.mp4 *.mov *.mkv *.webm *.mp3 *.wav *.m4a);;All Files (*.*)");

  openMedia(path);
}

void MainWindow::openProjectDialog()
{
  QString path = getNativeOpenFileName(this, "Open Project",
                                       suggestedProjectPath(m_project, m_projectPath),
                                       "Toaster Project (*.toaster);;All Files (*.*)");
  toaster_project_t *project;

  if (path.isEmpty())
    return;

  project = toaster_project_load(path.toUtf8().constData());
  if (!project) {
    QMessageBox::warning(this, "Open Project", "Failed to load project file.");
    appendLogLine(QString("Failed to load project: %1").arg(path));
    return;
  }

  loadProject(project, path);
}

void MainWindow::loadProject(toaster_project_t *project, const QString &projectPath)
{
  QString waveformError;
  QString mediaPath;

  clearProject();
  m_project = project;
  m_projectPath = projectPath;
  toaster_suggestion_list_clear(m_suggestions);

  mediaPath = QString::fromUtf8(toaster_project_get_media_path(m_project));
  if (!mediaPath.isEmpty()) {
    m_player->setSource(QUrl::fromLocalFile(mediaPath));
    if (!loadWaveformForMedia(mediaPath, &waveformError) && !waveformError.isEmpty())
      appendLogLine(waveformError);
  }

  rebuildAllViews();
  appendLogLine(QString("Loaded project: %1").arg(projectPath));
}

void MainWindow::saveProject()
{
  if (!m_projectPath.isEmpty()) {
    QString errorMessage;

    if (saveProjectToPath(m_projectPath, &errorMessage)) {
      appendLogLine(QString("Saved project: %1").arg(m_projectPath));
      return;
    }

    QMessageBox::warning(this, "Save Project", errorMessage);
    return;
  }

  saveProjectAs();
}

void MainWindow::saveProjectAs()
{
  QString path;

  if (!m_project)
    return;

  path = getNativeSaveFileName(this, "Save Project As", suggestedProjectPath(m_project, m_projectPath),
                               "Toaster Project (*.toaster)", "toaster");
  if (path.isEmpty())
    return;

  QString errorMessage;
  if (!saveProjectToPath(path, &errorMessage)) {
    QMessageBox::warning(this, "Save Project", errorMessage);
    return;
  }

  appendLogLine(QString("Saved project: %1").arg(path));
}

bool MainWindow::saveProjectToPath(const QString &path, QString *errorMessage)
{
  QString finalPath = path;

  if (!m_project) {
    if (errorMessage)
      *errorMessage = "No project is loaded.";
    return false;
  }

  if (finalPath.isEmpty()) {
    if (errorMessage)
      *errorMessage = "Project path is empty.";
    return false;
  }

  if (!finalPath.endsWith(".toaster", Qt::CaseInsensitive))
    finalPath += ".toaster";

  if (!toaster_project_save(m_project, finalPath.toUtf8().constData())) {
    if (errorMessage)
      *errorMessage = QString("Failed to save project file:\n%1").arg(finalPath);
    appendLogLine(QString("Failed to save project: %1").arg(finalPath));
    return false;
  }

  m_projectPath = finalPath;
  updateInspector();
  return true;
}

void MainWindow::transcribeMedia()
{
  QString title = m_transcribeAction ? m_transcribeAction->text() : "Transcribe Media";
  QString errorMessage;

  if (!confirmTranscriptReplacement(
        title, "Replace the current transcript with a fresh local whisper.cpp transcription?\n\n"
               "This keeps the current media path, but clears imported text, manual transcript edits, "
               "and pending suggestions.")) {
    return;
  }

  if (!transcribeCurrentMedia(true, &errorMessage)) {
    QMessageBox::warning(this, title, errorMessage);
    appendLogLine(errorMessage);
  }
}

QString MainWindow::locateWhisperModel() const
{
  QString envModel = qEnvironmentVariable("TOASTER_WHISPER_MODEL");
  QString appDataModel =
    QDir(QStandardPaths::writableLocation(QStandardPaths::AppDataLocation)).filePath(
      "models/ggml-tiny.en.bin");
  QStringList candidates = {
    envModel,
    QCoreApplication::applicationDirPath() + "/models/ggml-tiny.en.bin",
    appDataModel,
    "C:/msys64/mingw64/bin/models/ggml-tiny.en.bin",
  };

  for (const QString &candidate : candidates) {
    if (!candidate.isEmpty() && QFileInfo::exists(candidate))
      return candidate;
  }

  return appDataModel;
}

QString MainWindow::waveformCachePath(const QString &mediaPath) const
{
  QFileInfo info(mediaPath);
  QString cacheRoot = QStandardPaths::writableLocation(QStandardPaths::CacheLocation);
  QString cacheKey = info.absoluteFilePath() + "|" +
                     QString::number(info.lastModified().toMSecsSinceEpoch());
  QByteArray hash =
    QCryptographicHash::hash(cacheKey.toUtf8(), QCryptographicHash::Sha1).toHex();

  if (cacheRoot.isEmpty())
    cacheRoot = QCoreApplication::applicationDirPath();

  return QDir(cacheRoot + "/waveforms").filePath(QString::fromUtf8(hash) + ".png");
}

bool MainWindow::downloadWhisperModel(const QString &modelPath, QString *errorMessage)
{
  QString stdoutText;
  QString stderrText;
  QString targetPath = QDir::toNativeSeparators(modelPath);
  QString command =
    QString("Invoke-WebRequest -Uri '%1' -OutFile '%2'")
      .arg("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin?download=true")
      .arg(targetPath.replace("'", "''"));

  if (!QDir().mkpath(QFileInfo(modelPath).absolutePath())) {
    if (errorMessage)
      *errorMessage = QString("Failed to create model directory for:\n%1").arg(modelPath);
    return false;
  }

  appendLogLine("Downloading local whisper.cpp model (tiny.en)...");
  if (!runProcess("powershell.exe",
                  {"-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", command}, &stdoutText,
                  &stderrText)) {
    QFile::remove(modelPath);
    if (errorMessage) {
      *errorMessage = stderrText.isEmpty()
                        ? "Failed to download whisper.cpp model."
                        : QString("Failed to download whisper.cpp model.\n\n%1").arg(stderrText);
    }
    return false;
  }

  if (!QFileInfo::exists(modelPath) || QFileInfo(modelPath).size() == 0) {
    QFile::remove(modelPath);
    if (errorMessage)
      *errorMessage = "Downloaded whisper.cpp model was empty.";
    return false;
  }

  return true;
}

bool MainWindow::ensureWhisperModel(QString *modelPath, bool allowDownload,
                                    QString *errorMessage)
{
  QString candidate = locateWhisperModel();

  if (QFileInfo::exists(candidate)) {
    if (modelPath)
      *modelPath = candidate;
    return true;
  }

  if (!allowDownload) {
    if (errorMessage) {
      *errorMessage =
        "No local whisper.cpp model was found. Set TOASTER_WHISPER_MODEL or transcribe once with network access.";
    }
    return false;
  }

  if (!downloadWhisperModel(candidate, errorMessage))
    return false;

  if (modelPath)
    *modelPath = candidate;
  return true;
}

bool MainWindow::populateTranscriptFromWhisperJson(const QString &jsonPath, QString *errorMessage)
{
  QFile jsonFile(jsonPath);
  QJsonParseError parseError;
  QByteArray rawJson;
  QJsonDocument document;
  QJsonObject root;
  QJsonArray transcription;
  toaster_transcript_t *transcript;
  QString currentWord;
  qint64 currentStartUs = 0;
  qint64 currentEndUs = 0;
  bool haveCurrentWord = false;
  int parsedWords = 0;

  if (!m_project) {
    if (errorMessage)
      *errorMessage = "No project is loaded.";
    return false;
  }

  if (!jsonFile.open(QIODevice::ReadOnly | QIODevice::Text)) {
    if (errorMessage)
      *errorMessage = QString("Failed to open whisper transcript:\n%1").arg(jsonPath);
    return false;
  }

  rawJson = jsonFile.readAll();
  document = QJsonDocument::fromJson(rawJson, &parseError);
  if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
    if (errorMessage) {
      *errorMessage =
        QString("Failed to parse whisper transcript JSON: %1").arg(parseError.errorString());
    }
    return false;
  }

  root = document.object();
  transcription = root.value("transcription").toArray();
  if (transcription.isEmpty()) {
    if (errorMessage)
      *errorMessage = "whisper.cpp returned no transcription segments.";
    return false;
  }

  transcript = toaster_project_get_transcript(m_project);
  toaster_transcript_clear(transcript);
  toaster_transcript_clear_cut_spans(transcript);
  toaster_suggestion_list_clear(m_suggestions);

  auto flushWord = [&]() -> bool {
    if (!haveCurrentWord || currentWord.isEmpty())
      return true;

    if (currentEndUs <= currentStartUs)
      currentEndUs = currentStartUs + 100000;

    if (!toaster_transcript_add_word(transcript, currentWord.toUtf8().constData(), currentStartUs,
                                     currentEndUs)) {
      return false;
    }

    ++parsedWords;
    haveCurrentWord = false;
    currentWord.clear();
    currentStartUs = 0;
    currentEndUs = 0;
    return true;
  };

  for (const QJsonValue &segmentValue : transcription) {
    QJsonArray tokens = segmentValue.toObject().value("tokens").toArray();

    for (const QJsonValue &tokenValue : tokens) {
      QJsonObject tokenObject = tokenValue.toObject();
      QString rawToken = tokenObject.value("text").toString();
      QString normalizedToken = rawToken.trimmed();
      QJsonObject offsets = tokenObject.value("offsets").toObject();
      qint64 startUs = static_cast<qint64>(offsets.value("from").toDouble() * 1000.0);
      qint64 endUs = static_cast<qint64>(offsets.value("to").toDouble() * 1000.0);
      bool startsNewWord = rawToken.startsWith(' ');

      if (isSpecialWhisperToken(rawToken) || normalizedToken.isEmpty())
        continue;

      if (!haveCurrentWord) {
        currentWord = normalizedToken;
        currentStartUs = startUs;
        currentEndUs = std::max(startUs, endUs);
        haveCurrentWord = true;
        continue;
      }

      if (startsNewWord && !shouldAttachWhisperToken(normalizedToken)) {
        if (!flushWord()) {
          if (errorMessage)
            *errorMessage = "Failed to append transcribed word to transcript.";
          return false;
        }

        currentWord = normalizedToken;
        currentStartUs = startUs;
        currentEndUs = std::max(startUs, endUs);
        haveCurrentWord = true;
        continue;
      }

      currentWord += normalizedToken;
      currentEndUs = std::max(currentEndUs, std::max(startUs, endUs));
    }
  }

  if (!flushWord()) {
    if (errorMessage)
      *errorMessage = "Failed to append final transcribed word to transcript.";
    return false;
  }

  if (parsedWords == 0) {
    if (errorMessage)
      *errorMessage = "whisper.cpp returned no word-level tokens.";
    return false;
  }

  toaster_project_set_language(m_project,
                               root.value("result").toObject().value("language").toString("en")
                                 .toUtf8()
                                 .constData());
  rebuildAllViews();
  appendLogLine(QString("Transcribed media into %1 words.").arg(parsedWords));
  return true;
}

bool MainWindow::transcribeCurrentMedia(bool allowModelDownload, QString *errorMessage)
{
  QString mediaPath;
  QString whisperCliPath;
  QString whisperModelPath;
  QString ffmpegPath;
  QString stdoutText;
  QString stderrText;
  QTemporaryDir tempDir;
  QString wavPath;
  QString jsonBasePath;
  QString jsonPath;

  if (!m_project) {
    if (errorMessage)
      *errorMessage = "Open media first.";
    return false;
  }

  if (m_transcriptionInProgress) {
    if (errorMessage)
      *errorMessage = "Transcription is already running.";
    return false;
  }

  mediaPath = QString::fromUtf8(toaster_project_get_media_path(m_project));
  if (mediaPath.isEmpty()) {
    if (errorMessage)
      *errorMessage = "Open media first.";
    return false;
  }

  ScopedFlag transcriptionGuard(m_transcriptionInProgress);
  m_lastTranscriptionError.clear();

  whisperCliPath = locateTool("whisper-cli.exe");
  if (!QFileInfo::exists(whisperCliPath)) {
    if (errorMessage)
      *errorMessage = "whisper-cli.exe was not found in the deployed app or MSYS2 toolchain.";
    return false;
  }

  if (!ensureWhisperModel(&whisperModelPath, allowModelDownload, errorMessage))
    return false;

  if (!tempDir.isValid()) {
    if (errorMessage)
      *errorMessage = "Failed to create a temporary directory for transcription.";
    return false;
  }

  wavPath = tempDir.filePath("transcription.wav");
  jsonBasePath = tempDir.filePath("transcription");
  jsonPath = jsonBasePath + ".json";
  ffmpegPath = locateTool("ffmpeg.exe");

  appendLogLine("Extracting media audio for transcription...");
  if (!runProcess(ffmpegPath,
                  {"-y", "-i", mediaPath, "-vn", "-ac", "1", "-ar", "16000", "-c:a",
                   "pcm_s16le", wavPath},
                  &stdoutText, &stderrText)) {
    if (errorMessage) {
      *errorMessage = stderrText.isEmpty()
                        ? "Failed to extract audio for transcription."
                        : QString("Failed to extract audio for transcription.\n\n%1").arg(stderrText);
    }
    return false;
  }

  appendLogLine("Running local whisper.cpp transcription...");
  if (!runProcess(whisperCliPath,
                  {"-m", whisperModelPath, "-f", wavPath, "-oj", "-ojf", "-np", "-l", "en", "-of",
                   jsonBasePath},
                  &stdoutText, &stderrText)) {
    if (errorMessage) {
      *errorMessage = stderrText.isEmpty()
                        ? "whisper.cpp transcription failed."
                        : QString("whisper.cpp transcription failed.\n\n%1").arg(stderrText);
    }
    return false;
  }

  if (!populateTranscriptFromWhisperJson(jsonPath, errorMessage))
    return false;

  return true;
}

bool MainWindow::loadWaveformForMedia(const QString &mediaPath, QString *errorMessage)
{
  QString waveformPath;
  QString stdoutText;
  QString stderrText;
  QImage waveformImage;

  if (!m_waveformView)
    return false;

  if (mediaPath.isEmpty()) {
    m_waveformView->clear();
    return true;
  }

  waveformPath = waveformCachePath(mediaPath);
  if (!QFileInfo::exists(waveformPath) ||
      QFileInfo(waveformPath).lastModified() < QFileInfo(mediaPath).lastModified()) {
    QString ffmpegPath = locateTool("ffmpeg.exe");

    if (!QDir().mkpath(QFileInfo(waveformPath).absolutePath())) {
      if (errorMessage)
        *errorMessage = "Failed to create waveform cache directory.";
      m_waveformView->clear();
      return false;
    }

    appendLogLine("Generating waveform preview...");
    if (!runProcess(ffmpegPath,
                    {"-y", "-i", mediaPath, "-filter_complex",
                     "aformat=channel_layouts=mono,showwavespic=s=2400x220:colors=0x2d6cdf",
                     "-frames:v", "1", waveformPath},
                    &stdoutText, &stderrText)) {
      m_waveformView->clear();
      if (errorMessage) {
        *errorMessage = stderrText.isEmpty()
                          ? "Failed to render waveform preview."
                          : QString("Failed to render waveform preview.\n\n%1").arg(stderrText);
      }
      return false;
    }
  }

  waveformImage.load(waveformPath);
  if (waveformImage.isNull()) {
    m_waveformView->clear();
    if (errorMessage)
      *errorMessage = "Waveform image could not be loaded.";
    return false;
  }

  m_waveformView->setWaveformImage(waveformImage);
  return true;
}

void MainWindow::importTranscript()
{
  QFile file;
  QString path = getNativeOpenFileName(this, "Import Transcript Text", suggestedTranscriptPath(m_project),
                                       "Transcript Text (*.txt);;All Files (*.*)");
  QString errorMessage;
  QString importMessage;

  if (path.isEmpty())
    return;

  file.setFileName(path);
  if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
    QMessageBox::warning(this, "Import Transcript Text", "Failed to open transcript file.");
    appendLogLine(QString("Failed to import transcript: %1").arg(path));
    return;
  }

  importMessage =
    m_mediaDurationUs > 0
      ? "Import transcript text and replace the current transcript?\n\n"
          "Imported words are spread across the current media duration with estimated timings."
      : "Import transcript text and replace the current transcript?\n\n"
          "No media duration is loaded, so imported words will use estimated placeholder timings.";
  if (!confirmTranscriptReplacement("Import Transcript Text", importMessage))
    return;

  QTextStream stream(&file);
  if (!replaceTranscriptFromText(stream.readAll(), m_mediaDurationUs, &errorMessage)) {
    QMessageBox::warning(this, "Import Transcript Text", errorMessage);
    appendLogLine(errorMessage);
    return;
  }

  appendLogLine(QString("Imported transcript text with estimated timings: %1").arg(path));
}

bool MainWindow::replaceTranscriptFromText(const QString &text, qint64 duration_us,
                                           QString *errorMessage)
{
  QStringList tokens = splitTranscriptWords(text);
  toaster_transcript_t *transcript;
  int index;

  if (!ensureProject()) {
    if (errorMessage)
      *errorMessage = "Failed to create a project for transcript import.";
    return false;
  }
  if (tokens.isEmpty()) {
    if (errorMessage)
      *errorMessage = "Transcript text did not contain any words to import.";
    return false;
  }

  transcript = toaster_project_get_transcript(m_project);
  toaster_transcript_clear(transcript);
  toaster_transcript_clear_cut_spans(transcript);
  toaster_suggestion_list_clear(m_suggestions);

  if (duration_us <= 0)
    duration_us = static_cast<qint64>(tokens.size()) * 750000;

  for (index = 0; index < tokens.size(); ++index) {
    qint64 startUs = (duration_us * index) / tokens.size();
    qint64 endUs = (duration_us * (index + 1)) / tokens.size();
    if (endUs <= startUs)
      endUs = startUs + 100000;
    if (!toaster_transcript_add_word(transcript, tokens[index].toUtf8().constData(), startUs, endUs)) {
      if (errorMessage)
        *errorMessage = "Failed to import transcript text into the project.";
      return false;
    }
  }

  rebuildAllViews();
  return true;
}

bool MainWindow::confirmTranscriptReplacement(const QString &title, const QString &message)
{
  size_t wordCount =
    m_project ? toaster_transcript_word_count(toaster_project_get_transcript(m_project)) : 0;

  if (wordCount == 0)
    return true;

  return QMessageBox::question(
           this, title,
           QString("%1\n\nCurrent transcript words: %2")
             .arg(message)
             .arg(static_cast<qulonglong>(wordCount)),
           QMessageBox::Yes | QMessageBox::No, QMessageBox::No) == QMessageBox::Yes;
}

void MainWindow::focusTranscriptSearch()
{
  if (!m_transcriptSearchEdit || !m_transcriptSearchEdit->isEnabled()) {
    statusBar()->showMessage("Transcribe or import a transcript to enable search.", 4000);
    return;
  }

  if (m_transcriptDock) {
    m_transcriptDock->show();
    m_transcriptDock->raise();
  }
  m_transcriptSearchEdit->setFocus();
  m_transcriptSearchEdit->selectAll();
}

void MainWindow::findNextTranscriptMatch()
{
  int nextIndex;

  if (m_transcriptSearchMatches.isEmpty()) {
    focusTranscriptSearch();
    return;
  }

  nextIndex = (m_transcriptSearchIndex + 1) % m_transcriptSearchMatches.size();
  selectTranscriptSearchMatch(nextIndex);
}

void MainWindow::findPreviousTranscriptMatch()
{
  int previousIndex;

  if (m_transcriptSearchMatches.isEmpty()) {
    focusTranscriptSearch();
    return;
  }

  previousIndex = m_transcriptSearchIndex < 0
                    ? m_transcriptSearchMatches.size() - 1
                    : (m_transcriptSearchIndex + m_transcriptSearchMatches.size() - 1) %
                        m_transcriptSearchMatches.size();
  selectTranscriptSearchMatch(previousIndex);
}

void MainWindow::onTranscriptSearchChanged(const QString &text)
{
  Q_UNUSED(text);
  refreshTranscriptSearch(false);
}

bool MainWindow::selectTranscriptSearchMatch(int matchIndex)
{
  if (matchIndex < 0 || matchIndex >= m_transcriptSearchMatches.size())
    return false;

  m_transcriptSearchIndex = matchIndex;
  setTranscriptSelectionRange(m_transcriptSearchMatches.at(matchIndex).startRow,
                              m_transcriptSearchMatches.at(matchIndex).endRow, true);
  if (m_transcriptTable)
    m_transcriptTable->setFocus();

  updateTranscriptToolState(
    m_project ? toaster_transcript_word_count(toaster_project_get_transcript(m_project)) : 0,
    m_project ? QString::fromUtf8(toaster_project_get_media_path(m_project)) : QString());
  return true;
}

void MainWindow::updateTranscriptToolState(size_t wordCount, const QString &mediaPath)
{
  bool hasTranscript = wordCount > 0;
  bool hasMatches = !m_transcriptSearchMatches.isEmpty();
  QString query = m_transcriptSearchEdit ? m_transcriptSearchEdit->text().trimmed() : QString();
  QString transcribeLabel = hasTranscript ? "Re-Transcribe Media" : "Transcribe Media";
  QString transcribeTip =
    hasTranscript ? "Replace the current transcript with a fresh local whisper.cpp transcription."
                  : "Generate a timed transcript with local whisper.cpp.";

  if (m_transcribeAction) {
    m_transcribeAction->setText(transcribeLabel);
    m_transcribeAction->setToolTip(transcribeTip);
    m_transcribeAction->setStatusTip(transcribeTip);
    m_transcribeAction->setEnabled(!mediaPath.isEmpty() && !m_transcriptionInProgress);
  }
  if (m_importTranscriptAction) {
    m_importTranscriptAction->setToolTip(
      "Import plain transcript text and estimate word timings across the current media duration.");
    m_importTranscriptAction->setStatusTip(m_importTranscriptAction->toolTip());
    m_importTranscriptAction->setEnabled(!m_transcriptionInProgress);
  }
  if (m_focusTranscriptSearchAction)
    m_focusTranscriptSearchAction->setEnabled(hasTranscript);
  if (m_findNextAction)
    m_findNextAction->setEnabled(hasTranscript && hasMatches);
  if (m_findPreviousAction)
    m_findPreviousAction->setEnabled(hasTranscript && hasMatches);
  if (m_transcriptSearchEdit) {
    m_transcriptSearchEdit->setEnabled(hasTranscript);
    m_transcriptSearchEdit->setToolTip(
      hasTranscript
        ? "Search transcript text. Use Enter or Find Next/Previous to move between matches."
        : "Transcribe or import a transcript to enable search.");
  }
  if (m_transcriptSearchStatusLabel) {
    if (!hasTranscript) {
      m_transcriptSearchStatusLabel->setText("No transcript");
    } else if (query.isEmpty()) {
      m_transcriptSearchStatusLabel->setText(
        QString("%1 words").arg(static_cast<qulonglong>(wordCount)));
    } else if (!hasMatches) {
      m_transcriptSearchStatusLabel->setText("No matches");
    } else if (m_transcriptSearchIndex >= 0 && m_transcriptSearchIndex < m_transcriptSearchMatches.size()) {
      m_transcriptSearchStatusLabel->setText(
        QString("%1 of %2").arg(m_transcriptSearchIndex + 1).arg(m_transcriptSearchMatches.size()));
    } else {
      m_transcriptSearchStatusLabel->setText(QString("%1 matches").arg(m_transcriptSearchMatches.size()));
    }
  }
}

void MainWindow::clearTranscriptSearch()
{
  m_transcriptSearchMatches.clear();
  m_transcriptSearchIndex = -1;

  if (m_transcriptSearchEdit) {
    QSignalBlocker blocker(m_transcriptSearchEdit);
    m_transcriptSearchEdit->clear();
  }

  updateTranscriptToolState(
    m_project ? toaster_transcript_word_count(toaster_project_get_transcript(m_project)) : 0,
    m_project ? QString::fromUtf8(toaster_project_get_media_path(m_project)) : QString());
}

void MainWindow::refreshTranscriptSearch(bool preserveCurrentMatch)
{
  const toaster_transcript_t *transcript;
  QString query = m_transcriptSearchEdit ? m_transcriptSearchEdit->text().trimmed() : QString();
  QString mediaPath = m_project ? QString::fromUtf8(toaster_project_get_media_path(m_project)) : QString();
  QStringList queryTokens = splitTranscriptWords(query);
  int currentIndex = preserveCurrentMatch ? m_transcriptSearchIndex : -1;
  size_t wordCount =
    m_project ? toaster_transcript_word_count(toaster_project_get_transcript(m_project)) : 0;

  m_transcriptSearchMatches.clear();
  m_transcriptSearchIndex = -1;

  if (!m_project || queryTokens.isEmpty() || wordCount == 0) {
    updateTranscriptToolState(wordCount, mediaPath);
    return;
  }

  transcript = toaster_project_get_transcript_const(m_project);
  if (queryTokens.size() == 1) {
    for (int row = 0; row < static_cast<int>(wordCount); ++row) {
      toaster_word_t word;

      if (!toaster_transcript_get_word(transcript, static_cast<size_t>(row), &word))
        continue;
      if (QString::fromUtf8(word.text).contains(queryTokens.first(), Qt::CaseInsensitive))
        m_transcriptSearchMatches.append({row, row});
    }
  } else {
    int queryTokenCount = queryTokens.size();
    int lastStart = static_cast<int>(wordCount) - queryTokenCount;

    for (int startRow = 0; startRow <= lastStart; ++startRow) {
      bool matched = true;

      for (int offset = 0; offset < queryTokenCount; ++offset) {
        toaster_word_t word;

        if (!toaster_transcript_get_word(transcript, static_cast<size_t>(startRow + offset), &word) ||
            !QString::fromUtf8(word.text).contains(queryTokens.at(offset), Qt::CaseInsensitive)) {
          matched = false;
          break;
        }
      }

      if (matched)
        m_transcriptSearchMatches.append({startRow, startRow + queryTokenCount - 1});
    }
  }

  if (m_transcriptSearchMatches.isEmpty()) {
    updateTranscriptToolState(wordCount, mediaPath);
    return;
  }

  if (currentIndex < 0 || currentIndex >= m_transcriptSearchMatches.size())
    currentIndex = 0;
  selectTranscriptSearchMatch(currentIndex);
}

void MainWindow::exportMedia()
{
  QString errorMessage;
  QString mediaPath;
  QString outputPath;

  if (!m_project)
    return;

  mediaPath = QString::fromUtf8(toaster_project_get_media_path(m_project));
  if (mediaPath.isEmpty()) {
    QMessageBox::information(this, "Export Media", "Open media first.");
    return;
  }

  outputPath = getNativeSaveFileName(this, "Export Edited Media", suggestedExportPath(m_project),
                                     "MP4 Video (*.mp4)", "mp4");
  if (outputPath.isEmpty())
    return;
  if (!exportMediaToPath(outputPath, &errorMessage)) {
    QMessageBox::warning(this, "Export Media", errorMessage);
    return;
  }

  appendLogLine(QString("Exported edited media: %1").arg(outputPath));
  QMessageBox::information(this, "Export Media", QString("Export complete:\n%1").arg(outputPath));
}

void MainWindow::exportCaptions()
{
  QString errorMessage;
  QString outputPath;
  QString finalOutputPath;
  toaster_caption_format_t format;
  toaster_transcript_t *transcript;

  if (!m_project)
    return;

  transcript = toaster_project_get_transcript(m_project);
  if (toaster_transcript_word_count(transcript) == 0) {
    QMessageBox::information(this, "Export Captions", "Transcribe or import a transcript first.");
    return;
  }

  outputPath = getNativeSaveFileName(this, "Export Captions",
                                     suggestedCaptionPath(m_project, m_projectPath),
                                     "Caption Files (*.srt *.vtt);;SRT Captions (*.srt);;WebVTT Captions (*.vtt)",
                                     "srt");
  if (outputPath.isEmpty())
    return;

  format = captionFormatForPath(outputPath);
  finalOutputPath = outputPath;
  if (format == TOASTER_CAPTION_FORMAT_VTT) {
    if (!finalOutputPath.endsWith(".vtt", Qt::CaseInsensitive))
      finalOutputPath += ".vtt";
  } else if (!finalOutputPath.endsWith(".srt", Qt::CaseInsensitive)) {
    finalOutputPath += ".srt";
  }

  if (!exportCaptionsToPath(outputPath, format, &errorMessage)) {
    QMessageBox::warning(this, "Export Captions", errorMessage);
    return;
  }

  appendLogLine(QString("Exported captions: %1").arg(finalOutputPath));
  QMessageBox::information(this, "Export Captions",
                           QString("Export complete:\n%1").arg(finalOutputPath));
}

void MainWindow::exportScript()
{
  QString errorMessage;
  QString outputPath;
  QString finalOutputPath;
  toaster_transcript_t *transcript;

  if (!m_project)
    return;

  transcript = toaster_project_get_transcript(m_project);
  if (toaster_transcript_word_count(transcript) == 0) {
    QMessageBox::information(this, "Export Script", "Transcribe or import a transcript first.");
    return;
  }

  outputPath = getNativeSaveFileName(this, "Export Script",
                                     suggestedScriptPath(m_project, m_projectPath),
                                     "Plain Text (*.txt)", "txt");
  if (outputPath.isEmpty())
    return;

  finalOutputPath = outputPath;
  if (!finalOutputPath.endsWith(".txt", Qt::CaseInsensitive))
    finalOutputPath += ".txt";

  if (!exportScriptToPath(outputPath, &errorMessage)) {
    QMessageBox::warning(this, "Export Script", errorMessage);
    return;
  }

  appendLogLine(QString("Exported script: %1").arg(finalOutputPath));
  QMessageBox::information(this, "Export Script",
                           QString("Export complete:\n%1").arg(finalOutputPath));
}

bool MainWindow::exportMediaToPath(const QString &outputPath, QString *errorMessage)
{
  QString finalOutputPath = outputPath;
  QString mediaPath;
  QString ffmpeg;
  QString filterComplex;
  QStringList arguments;
  QString stdoutText;
  QString stderrText;
  QString audioLabel = "[0:a]";
  QString concatInputs;
  size_t silenceCount;
  size_t segmentCount;
  int segmentIndex;
  toaster_transcript_t *transcript;

  if (!m_project) {
    if (errorMessage)
      *errorMessage = "No project is loaded.";
    return false;
  }

  mediaPath = QString::fromUtf8(toaster_project_get_media_path(m_project));
  if (mediaPath.isEmpty()) {
    if (errorMessage)
      *errorMessage = "Open media first.";
    return false;
  }

  if (finalOutputPath.isEmpty()) {
    if (errorMessage)
      *errorMessage = "Output path is empty.";
    return false;
  }

  if (!finalOutputPath.endsWith(".mp4", Qt::CaseInsensitive))
    finalOutputPath += ".mp4";

  transcript = toaster_project_get_transcript(m_project);
  segmentCount = toaster_transcript_keep_segment_count(transcript);
  if (segmentCount == 0) {
    if (errorMessage)
      *errorMessage = "No keep segments remain to export.";
    return false;
  }

  silenceCount = toaster_transcript_silenced_span_count(transcript);
  for (size_t index = 0; index < silenceCount; ++index) {
    toaster_time_range_t range;
    QString outLabel;

    if (!toaster_transcript_get_silenced_span(transcript, index, &range))
      continue;

    outLabel = QString("[am%1]").arg(index);
    filterComplex += QString("%1volume=enable='between(t,%2,%3)':volume=0%4;")
                       .arg(audioLabel)
                       .arg(static_cast<double>(range.start_us) / 1000000.0, 0, 'f', 6)
                       .arg(static_cast<double>(range.end_us) / 1000000.0, 0, 'f', 6)
                       .arg(outLabel);
    audioLabel = outLabel;
  }

  for (segmentIndex = 0; segmentIndex < static_cast<int>(segmentCount); ++segmentIndex) {
    toaster_time_range_t segment;

    if (!toaster_transcript_get_keep_segment(transcript, static_cast<size_t>(segmentIndex), &segment))
      continue;

    filterComplex += QString("[0:v]trim=start=%1:end=%2,setpts=PTS-STARTPTS[v%3];")
                       .arg(static_cast<double>(segment.start_us) / 1000000.0, 0, 'f', 6)
                       .arg(static_cast<double>(segment.end_us) / 1000000.0, 0, 'f', 6)
                       .arg(segmentIndex);
    filterComplex += QString("%1atrim=start=%2:end=%3,asetpts=PTS-STARTPTS[a%4];")
                       .arg(audioLabel)
                       .arg(static_cast<double>(segment.start_us) / 1000000.0, 0, 'f', 6)
                       .arg(static_cast<double>(segment.end_us) / 1000000.0, 0, 'f', 6)
                       .arg(segmentIndex);
    concatInputs += QString("[v%1][a%1]").arg(segmentIndex);
  }

  filterComplex += QString("%1concat=n=%2:v=1:a=1[outv][outa]")
                     .arg(concatInputs)
                     .arg(static_cast<int>(segmentCount));

  ffmpeg = locateTool("ffmpeg.exe");
  arguments << "-y"
            << "-i" << mediaPath
            << "-filter_complex" << filterComplex
            << "-map" << "[outv]"
            << "-map" << "[outa]"
            << "-c:v" << "libx264"
            << "-preset" << "veryfast"
            << "-crf" << "18"
            << "-c:a" << "aac"
            << "-b:a" << "192k"
            << "-movflags" << "+faststart"
            << finalOutputPath;

  if (!runProcess(ffmpeg, arguments, &stdoutText, &stderrText)) {
    if (errorMessage) {
      *errorMessage = stderrText.isEmpty()
                        ? QString("FFmpeg export failed while running %1.").arg(ffmpeg)
                        : QString("FFmpeg export failed.\n\n%1").arg(stderrText);
    }
    appendLogLine("FFmpeg export failed.");
    return false;
  }

  return true;
}

bool MainWindow::exportCaptionsToPath(const QString &outputPath, toaster_caption_format_t format,
                                      QString *errorMessage)
{
  QString finalOutputPath = outputPath;
  toaster_transcript_t *transcript;

  if (!m_project) {
    if (errorMessage)
      *errorMessage = "No project is loaded.";
    return false;
  }

  transcript = toaster_project_get_transcript(m_project);
  if (toaster_transcript_word_count(transcript) == 0) {
    if (errorMessage)
      *errorMessage = "Transcribe or import a transcript first.";
    return false;
  }

  if (finalOutputPath.isEmpty()) {
    if (errorMessage)
      *errorMessage = "Output path is empty.";
    return false;
  }

  if (format == TOASTER_CAPTION_FORMAT_VTT) {
    if (!finalOutputPath.endsWith(".vtt", Qt::CaseInsensitive))
      finalOutputPath += ".vtt";
  } else {
    if (!finalOutputPath.endsWith(".srt", Qt::CaseInsensitive))
      finalOutputPath += ".srt";
  }

  if (!toaster_transcript_export_captions(transcript, finalOutputPath.toUtf8().constData(), format)) {
    if (errorMessage)
      *errorMessage = QString("Failed to export captions:\n%1").arg(finalOutputPath);
    appendLogLine(QString("Caption export failed: %1").arg(finalOutputPath));
    return false;
  }

  return true;
}

bool MainWindow::exportScriptToPath(const QString &outputPath, QString *errorMessage)
{
  QString finalOutputPath = outputPath;
  toaster_transcript_t *transcript;

  if (!m_project) {
    if (errorMessage)
      *errorMessage = "No project is loaded.";
    return false;
  }

  transcript = toaster_project_get_transcript(m_project);
  if (toaster_transcript_word_count(transcript) == 0) {
    if (errorMessage)
      *errorMessage = "Transcribe or import a transcript first.";
    return false;
  }

  if (finalOutputPath.isEmpty()) {
    if (errorMessage)
      *errorMessage = "Output path is empty.";
    return false;
  }

  if (!finalOutputPath.endsWith(".txt", Qt::CaseInsensitive))
    finalOutputPath += ".txt";

  if (!toaster_transcript_export_script(transcript, finalOutputPath.toUtf8().constData())) {
    if (errorMessage)
      *errorMessage = QString("Failed to export script:\n%1").arg(finalOutputPath);
    appendLogLine(QString("Script export failed: %1").arg(finalOutputPath));
    return false;
  }

  return true;
}

void MainWindow::analyzeCleanup()
{
  toaster_transcript_t *transcript;

  if (!m_project || !m_suggestions)
    return;

  transcript = toaster_project_get_transcript(m_project);
  if (toaster_transcript_word_count(transcript) == 0) {
    appendLogLine("No transcript words to analyze.");
    return;
  }

  toaster_suggestion_list_clear(m_suggestions);
  toaster_detect_fillers(transcript, m_suggestions);
  toaster_detect_pauses(transcript, m_suggestions, 400000, 150000);

  rebuildSuggestionList();
  updateInspector();
  appendLogLine(QString("Generated %1 cleanup suggestions.")
                  .arg(static_cast<qulonglong>(toaster_suggestion_list_count(m_suggestions))));
}

void MainWindow::applySelectedSuggestion()
{
  QListWidgetItem *item = m_suggestionList->currentItem();

  if (!item)
    return;

  if (applySuggestion(item->data(Qt::UserRole).toULongLong()))
    analyzeCleanup();
}

void MainWindow::applyAllSuggestions()
{
  size_t count = toaster_suggestion_list_count(m_suggestions);

  for (size_t index = 0; index < count; ++index)
    applySuggestion(index);

  analyzeCleanup();
}

bool MainWindow::applySuggestion(size_t suggestionIndex)
{
  toaster_suggestion_t suggestion;
  toaster_transcript_t *transcript;

  if (!m_project || !toaster_suggestion_list_get(m_suggestions, suggestionIndex, &suggestion))
    return false;

  transcript = toaster_project_get_transcript(m_project);
  switch (suggestion.kind) {
  case TOASTER_SUGGESTION_DELETE_FILLER:
    toaster_transcript_delete_range(transcript, suggestion.start_index, suggestion.end_index);
    break;
  case TOASTER_SUGGESTION_SILENCE_FILLER:
    toaster_transcript_silence_range(transcript, suggestion.start_index, suggestion.end_index);
    break;
  case TOASTER_SUGGESTION_SHORTEN_PAUSE: {
    qint64 cutEnd = suggestion.end_us - suggestion.replacement_duration_us;
    if (cutEnd > suggestion.start_us)
      toaster_transcript_add_cut_span(transcript, suggestion.start_us, cutEnd);
    break;
  }
  }

  rebuildAllViews();
  appendLogLine(QString("Applied suggestion: %1").arg(QString::fromUtf8(suggestion.reason)));
  return true;
}

void MainWindow::deleteSelection()
{
  QList<int> rows = selectedRows();
  toaster_transcript_t *transcript;

  if (!m_project || rows.isEmpty())
    return;

  transcript = toaster_project_get_transcript(m_project);
  for (int row : rows)
    toaster_transcript_delete_range(transcript, static_cast<size_t>(row), static_cast<size_t>(row));

  rebuildAllViews();
  appendLogLine(QString("Deleted %1 selected words.").arg(rows.size()));
}

void MainWindow::restoreSelection()
{
  QList<int> rows = selectedRows();
  toaster_transcript_t *transcript;

  if (!m_project || rows.isEmpty())
    return;

  transcript = toaster_project_get_transcript(m_project);
  for (int row : rows)
    toaster_transcript_restore_range(transcript, static_cast<size_t>(row), static_cast<size_t>(row));

  rebuildAllViews();
  appendLogLine(QString("Restored %1 selected words.").arg(rows.size()));
}

void MainWindow::silenceSelection()
{
  QList<int> rows = selectedRows();
  toaster_transcript_t *transcript;

  if (!m_project || rows.isEmpty())
    return;

  transcript = toaster_project_get_transcript(m_project);
  for (int row : rows)
    toaster_transcript_silence_range(transcript, static_cast<size_t>(row), static_cast<size_t>(row));

  rebuildAllViews();
  appendLogLine(QString("Silenced %1 selected words.").arg(rows.size()));
}

void MainWindow::unsilenceSelection()
{
  QList<int> rows = selectedRows();
  toaster_transcript_t *transcript;

  if (!m_project || rows.isEmpty())
    return;

  transcript = toaster_project_get_transcript(m_project);
  for (int row : rows)
    toaster_transcript_unsilence_range(transcript, static_cast<size_t>(row),
                                       static_cast<size_t>(row));

  rebuildAllViews();
  appendLogLine(QString("Unsilenced %1 selected words.").arg(rows.size()));
}

void MainWindow::onTranscriptCellChanged(int row, int column)
{
  toaster_word_t word;
  bool ok = false;
  qint64 newValueUs = 0;
  QString cellText;
  toaster_transcript_t *transcript;

  if (m_updatingTranscriptTable || !m_project)
    return;

  transcript = toaster_project_get_transcript(m_project);
  if (!toaster_transcript_get_word(transcript, static_cast<size_t>(row), &word))
    return;

  cellText = m_transcriptTable->item(row, column) ? m_transcriptTable->item(row, column)->text() : "";
  if (column == 0) {
    toaster_transcript_set_word_text(transcript, static_cast<size_t>(row),
                                     cellText.toUtf8().constData());
  } else if (column == 1 || column == 2) {
    newValueUs = static_cast<qint64>(cellText.toDouble(&ok) * 1000000.0);
    if (!ok) {
      rebuildTranscriptTable();
      return;
    }

    if (column == 1)
      toaster_transcript_set_word_times(transcript, static_cast<size_t>(row), newValueUs,
                                        word.end_us);
    else
      toaster_transcript_set_word_times(transcript, static_cast<size_t>(row), word.start_us,
                                        newValueUs);
  }

  rebuildAllViews();
}

void MainWindow::onTranscriptSelectionChanged()
{
  QList<int> rows;

  if (m_updatingTranscriptTable)
    return;

  rows = selectedRows();
  if (!rows.isEmpty()) {
    for (int index = 0; index < m_transcriptSearchMatches.size(); ++index) {
      const MainWindow::TranscriptSearchMatch &match = m_transcriptSearchMatches.at(index);

      if (match.startRow == rows.first() && match.endRow == rows.last()) {
        m_transcriptSearchIndex = index;
        break;
      }
    }
  }

  updateTranscriptToolState(
    m_project ? toaster_transcript_word_count(toaster_project_get_transcript(m_project)) : 0,
    m_project ? QString::fromUtf8(toaster_project_get_media_path(m_project)) : QString());
  syncWaveformSelectionFromContext();
  syncSuggestionSelectionForTranscriptSelection();
  syncPlaybackToSelection();
}

void MainWindow::onSuggestionActivated()
{
  applySelectedSuggestion();
}

void MainWindow::onSuggestionSelectionChanged()
{
  QListWidgetItem *item = m_suggestionList ? m_suggestionList->currentItem() : nullptr;
  toaster_suggestion_t suggestion;

  if (!m_project || !item || !m_suggestions)
    return;

  if (!toaster_suggestion_list_get(m_suggestions, static_cast<size_t>(item->data(Qt::UserRole).toULongLong()),
                                   &suggestion)) {
    return;
  }

  setTranscriptSelectionRange(static_cast<int>(suggestion.start_index),
                              static_cast<int>(suggestion.end_index), false);
  m_player->setPosition(suggestion.start_us / 1000);
}

void MainWindow::onDurationChanged(qint64 duration_ms)
{
  toaster_transcript_t *transcript;

  m_mediaDurationUs = duration_ms * 1000;
  m_positionSlider->setRange(0, static_cast<int>(qMin<qint64>(
                                    duration_ms, static_cast<qint64>(std::numeric_limits<int>::max()))));
  if (m_waveformView)
    m_waveformView->setDurationUs(m_mediaDurationUs);

  if (m_project) {
    transcript = toaster_project_get_transcript(m_project);
    if (toaster_transcript_word_count(transcript) > 0)
      rebuildWaveformView();
  }

  updateInspector();
}

void MainWindow::onPositionChanged(qint64 position_ms)
{
  qint64 positionUs = position_ms * 1000;
  QList<int> rows;

  if (m_updatingSlider)
    return;

  QSignalBlocker blocker(m_positionSlider);
  m_positionSlider->setValue(static_cast<int>(position_ms));
  if (m_waveformView)
    m_waveformView->setPlayheadUs(positionUs);

  rows = selectedRows();
  updateActiveTranscriptRow(transcriptRowForPosition(positionUs));
  if (m_activeTranscriptRow >= 0 &&
      (rows.isEmpty() || m_player->playbackState() == QMediaPlayer::PlayingState)) {
    scrollTranscriptRowIntoView(m_activeTranscriptRow);
  }

  if (rows.isEmpty() || m_player->playbackState() == QMediaPlayer::PlayingState)
    syncSuggestionSelectionForPosition(positionUs);
}

void MainWindow::onSliderMoved(int value)
{
  m_player->setPosition(value);
}

void MainWindow::onWaveformSeekRequested(qint64 positionUs)
{
  toaster_suggestion_t suggestion;
  int row;
  int suggestionRow;

  m_player->setPosition(positionUs / 1000);

  row = transcriptRowForPosition(positionUs);
  if (row >= 0) {
    setTranscriptSelectionRange(row, row, false);
    return;
  }

  suggestionRow = suggestionRowForPosition(positionUs);
  if (suggestionRow < 0 ||
      !toaster_suggestion_list_get(m_suggestions, static_cast<size_t>(suggestionRow), &suggestion)) {
    return;
  }

  setTranscriptSelectionRange(static_cast<int>(suggestion.start_index),
                              static_cast<int>(suggestion.end_index), false);
}

void MainWindow::rebuildAllViews()
{
  rebuildTranscriptTable();
  rebuildWaveformView();
  rebuildSuggestionList();
  updateInspector();
}

void MainWindow::rebuildTranscriptTable()
{
  size_t wordCount = m_project ? toaster_transcript_word_count(toaster_project_get_transcript(m_project))
                               : 0;

  m_updatingTranscriptTable = true;
  {
    QSignalBlocker blocker(m_transcriptTable);
    m_transcriptTable->setRowCount(static_cast<int>(wordCount));

    for (int row = 0; row < static_cast<int>(wordCount); ++row) {
      toaster_word_t word;
      QTableWidgetItem *wordItem;
      QTableWidgetItem *startItem;
      QTableWidgetItem *endItem;
      QTableWidgetItem *stateItem;

      toaster_transcript_get_word(toaster_project_get_transcript(m_project), static_cast<size_t>(row), &word);
      wordItem = new QTableWidgetItem(QString::fromUtf8(word.text));
      startItem = new QTableWidgetItem(formatSecondsCell(word.start_us));
      endItem = new QTableWidgetItem(formatSecondsCell(word.end_us));
      stateItem = new QTableWidgetItem(wordStateLabel(word));
      stateItem->setFlags(stateItem->flags() & ~Qt::ItemIsEditable);

      m_transcriptTable->setItem(row, 0, wordItem);
      m_transcriptTable->setItem(row, 1, startItem);
      m_transcriptTable->setItem(row, 2, endItem);
      m_transcriptTable->setItem(row, 3, stateItem);
      updateTranscriptRowVisualState(row);
    }
  }
  m_updatingTranscriptTable = false;
  refreshTranscriptSearch(true);
}

void MainWindow::rebuildWaveformView()
{
  QVector<toaster_time_range_t> cutRanges;
  QVector<toaster_time_range_t> deletedRanges;
  QVector<toaster_time_range_t> silencedRanges;
  size_t cutCount = 0;
  size_t deletedCount = 0;
  size_t silencedCount = 0;
  toaster_time_range_t bounds;

  if (!m_waveformView)
    return;

  if (m_project) {
    toaster_transcript_t *transcript = toaster_project_get_transcript(m_project);
    deletedCount = toaster_transcript_deleted_span_count(transcript);
    cutCount = toaster_transcript_cut_span_count(transcript);
    silencedCount = toaster_transcript_silenced_span_count(transcript);
    for (size_t index = 0; index < deletedCount; ++index) {
      toaster_time_range_t range;
      if (toaster_transcript_get_deleted_span(transcript, index, &range))
        deletedRanges.append(range);
    }

    for (size_t index = 0; index < cutCount; ++index) {
      toaster_time_range_t range;
      if (toaster_transcript_get_cut_span(transcript, index, &range))
        cutRanges.append(range);
    }

    for (size_t index = 0; index < silencedCount; ++index) {
      toaster_time_range_t range;
      if (toaster_transcript_get_silenced_span(transcript, index, &range))
        silencedRanges.append(range);
    }

    if (m_mediaDurationUs <= 0 && toaster_transcript_get_bounds(transcript, &bounds))
      m_waveformView->setDurationUs(bounds.end_us);
    else
      m_waveformView->setDurationUs(m_mediaDurationUs);
  } else {
    m_waveformView->clearSelectedRange();
    m_waveformView->setDurationUs(m_mediaDurationUs);
  }

  m_waveformView->setDeletedRanges(deletedRanges);
  m_waveformView->setCutRanges(cutRanges);
  m_waveformView->setSilencedRanges(silencedRanges);
  syncWaveformSelectionFromContext();
}

void MainWindow::rebuildSuggestionList()
{
  size_t suggestionCount = toaster_suggestion_list_count(m_suggestions);
  QList<int> rows = selectedRows();

  m_suggestionList->clear();
  for (size_t index = 0; index < suggestionCount; ++index) {
    toaster_suggestion_t suggestion;
    auto *item = new QListWidgetItem();

    if (!toaster_suggestion_list_get(m_suggestions, index, &suggestion))
      continue;

    item->setText(QString("[%1] %2 - %3 to %4")
                    .arg(suggestionKindLabel(suggestion.kind))
                    .arg(QString::fromUtf8(suggestion.reason))
                    .arg(formatMicros(suggestion.start_us))
                    .arg(formatMicros(suggestion.end_us)));
    item->setData(Qt::UserRole, QVariant::fromValue<qulonglong>(index));
    m_suggestionList->addItem(item);
  }

  syncSuggestionSelectionForTranscriptSelection();
  if (rows.isEmpty() && !m_suggestionList->currentItem())
    syncSuggestionSelectionForPosition(m_player->position() * 1000);
}

void MainWindow::syncWaveformSelectionFromContext()
{
  toaster_transcript_t *transcript;
  toaster_word_t firstWord;
  toaster_word_t lastWord;
  QList<int> rows = selectedRows();
  int startRow = -1;
  int endRow = -1;

  if (!m_waveformView) {
    return;
  } else if (!m_project) {
    m_waveformView->clearSelectedRange();
    return;
  }

  if (!rows.isEmpty()) {
    startRow = rows.first();
    endRow = rows.last();
  } else if (m_activeTranscriptRow >= 0) {
    startRow = m_activeTranscriptRow;
    endRow = m_activeTranscriptRow;
  } else {
    m_waveformView->clearSelectedRange();
    return;
  }

  transcript = toaster_project_get_transcript(m_project);
  if (toaster_transcript_get_word(transcript, static_cast<size_t>(startRow), &firstWord) &&
      toaster_transcript_get_word(transcript, static_cast<size_t>(endRow), &lastWord)) {
    m_waveformView->setSelectedRange({firstWord.start_us, lastWord.end_us});
  } else {
    m_waveformView->clearSelectedRange();
  }
}

void MainWindow::setTranscriptSelectionRange(int startRow, int endRow, bool seekPlayback)
{
  QItemSelectionModel *selectionModel;

  if (!m_transcriptTable || !m_transcriptTable->selectionModel() || startRow < 0 || endRow < startRow ||
      endRow >= m_transcriptTable->rowCount()) {
    return;
  }

  selectionModel = m_transcriptTable->selectionModel();
  {
    QSignalBlocker blocker(selectionModel);
    selectionModel->clearSelection();
    for (int row = startRow; row <= endRow; ++row) {
      selectionModel->select(m_transcriptTable->model()->index(row, 0),
                             QItemSelectionModel::Select | QItemSelectionModel::Rows);
    }
    selectionModel->setCurrentIndex(m_transcriptTable->model()->index(startRow, 0),
                                    QItemSelectionModel::NoUpdate);
  }

  scrollTranscriptRowIntoView(startRow);
  syncWaveformSelectionFromContext();
  syncSuggestionSelectionForTranscriptSelection();
  if (seekPlayback)
    syncPlaybackToSelection();
}

void MainWindow::setCurrentSuggestionRow(int row)
{
  if (!m_suggestionList || m_suggestionList->currentRow() == row)
    return;

  QSignalBlocker blocker(m_suggestionList);
  if (row >= 0 && row < m_suggestionList->count()) {
    m_suggestionList->setCurrentRow(row);
  } else {
    m_suggestionList->clearSelection();
    m_suggestionList->setCurrentItem(nullptr);
  }
}

void MainWindow::syncSuggestionSelectionForTranscriptSelection()
{
  QList<int> rows = selectedRows();

  if (rows.isEmpty()) {
    setCurrentSuggestionRow(-1);
    return;
  }

  setCurrentSuggestionRow(suggestionRowForTranscriptRange(rows.first(), rows.last()));
}

void MainWindow::syncSuggestionSelectionForPosition(qint64 positionUs)
{
  setCurrentSuggestionRow(suggestionRowForPosition(positionUs));
}

void MainWindow::updateActiveTranscriptRow(int row)
{
  int previousRow = m_activeTranscriptRow;

  if (previousRow == row)
    return;

  m_activeTranscriptRow = row;
  updateTranscriptRowVisualState(previousRow);
  updateTranscriptRowVisualState(m_activeTranscriptRow);
  if (selectedRows().isEmpty())
    syncWaveformSelectionFromContext();
}

void MainWindow::updateTranscriptRowVisualState(int row)
{
  toaster_word_t word;
  QColor background;
  bool active;

  if (row < 0 || !m_project || !m_transcriptTable || row >= m_transcriptTable->rowCount() ||
      !toaster_transcript_get_word(toaster_project_get_transcript(m_project), static_cast<size_t>(row), &word)) {
    return;
  }

  active = row == m_activeTranscriptRow;
  background = transcriptRowBackground(word, active);
  for (int column = 0; column < m_transcriptTable->columnCount(); ++column) {
    QTableWidgetItem *item = m_transcriptTable->item(row, column);
    QFont font;

    if (!item)
      continue;

    font = item->font();
    font.setStrikeOut(word.deleted);
    font.setBold(active);
    item->setFont(font);
    item->setBackground(background.isValid() ? QBrush(background) : QBrush());
  }
}

void MainWindow::scrollTranscriptRowIntoView(int row)
{
  QTableWidgetItem *item;
  QRect itemRect;
  QRect viewportRect;

  if (!m_transcriptTable || row < 0 || row >= m_transcriptTable->rowCount())
    return;

  item = m_transcriptTable->item(row, 0);
  if (!item)
    return;

  itemRect = m_transcriptTable->visualItemRect(item);
  viewportRect = m_transcriptTable->viewport()->rect().adjusted(0, 8, 0, -8);
  if (viewportRect.contains(itemRect.topLeft()) && viewportRect.contains(itemRect.bottomLeft()))
    return;

  m_transcriptTable->scrollToItem(item, QAbstractItemView::PositionAtCenter);
}

void MainWindow::updateInspector()
{
  QString projectName = m_projectPath.isEmpty() ? "Unsaved project" : QFileInfo(m_projectPath).fileName();
  QString mediaPath = m_project ? QString::fromUtf8(toaster_project_get_media_path(m_project)) : QString();
  size_t wordCount = 0;
  size_t deletedCount = 0;
  size_t silencedCount = 0;
  size_t cutCount = 0;
  size_t suggestionCount = toaster_suggestion_list_count(m_suggestions);

  if (m_project) {
    toaster_transcript_t *transcript = toaster_project_get_transcript(m_project);
    wordCount = toaster_transcript_word_count(transcript);
    deletedCount = toaster_transcript_deleted_span_count(transcript);
    silencedCount = toaster_transcript_silenced_span_count(transcript);
    cutCount = toaster_transcript_cut_span_count(transcript);
    m_languageLabel->setText(QString::fromUtf8(toaster_project_get_language(m_project)));
  } else {
    m_languageLabel->setText("en-US");
  }

  m_projectLabel->setText(projectName);
  m_mediaLabel->setText(mediaPath.isEmpty() ? "No media loaded" : mediaPath);
  m_durationLabel->setText(formatMicros(m_mediaDurationUs));
  m_statsLabel->setText(QString("Words: %1\nDeleted spans: %2\nSilenced spans: %3\nCuts: %4\nSuggestions: %5")
                          .arg(static_cast<qulonglong>(wordCount))
                          .arg(static_cast<qulonglong>(deletedCount))
                          .arg(static_cast<qulonglong>(silencedCount))
                          .arg(static_cast<qulonglong>(cutCount))
                          .arg(static_cast<qulonglong>(suggestionCount)));

  if (m_saveProjectAction)
    m_saveProjectAction->setEnabled(m_project != nullptr);
  if (m_exportMediaAction)
    m_exportMediaAction->setEnabled(!mediaPath.isEmpty() && wordCount > 0);
  if (m_exportCaptionsAction)
    m_exportCaptionsAction->setEnabled(wordCount > 0);
  if (m_exportScriptAction)
    m_exportScriptAction->setEnabled(wordCount > 0);
  if (m_analyzeAction)
    m_analyzeAction->setEnabled(wordCount > 0);
  updateTranscriptToolState(wordCount, mediaPath);
}

QList<int> MainWindow::selectedRows() const
{
  QList<int> rows;

  if (!m_transcriptTable || !m_transcriptTable->selectionModel())
    return rows;

  for (const QModelIndex &index : m_transcriptTable->selectionModel()->selectedRows())
    rows.append(index.row());

  std::sort(rows.begin(), rows.end());
  rows.erase(std::unique(rows.begin(), rows.end()), rows.end());
  return rows;
}

int MainWindow::transcriptRowForPosition(qint64 positionUs) const
{
  const toaster_transcript_t *transcript;
  size_t wordCount;

  if (!m_project)
    return -1;

  transcript = toaster_project_get_transcript_const(m_project);
  wordCount = toaster_transcript_word_count(transcript);
  for (size_t index = 0; index < wordCount; ++index) {
    toaster_word_t word;
    bool lastWord = index + 1 == wordCount;

    if (!toaster_transcript_get_word(transcript, index, &word))
      continue;
    if (positionUs < word.start_us)
      break;
    if (positionUs >= word.start_us && (positionUs < word.end_us || (lastWord && positionUs <= word.end_us)))
      return static_cast<int>(index);
  }

  return -1;
}

int MainWindow::suggestionRowForTranscriptRange(int startRow, int endRow) const
{
  int overlapMatch = -1;
  size_t count = toaster_suggestion_list_count(m_suggestions);

  for (size_t index = 0; index < count; ++index) {
    toaster_suggestion_t suggestion;

    if (!toaster_suggestion_list_get(m_suggestions, index, &suggestion))
      continue;

    if (static_cast<int>(suggestion.start_index) == startRow &&
        static_cast<int>(suggestion.end_index) == endRow) {
      return static_cast<int>(index);
    }

    if (static_cast<int>(suggestion.end_index) < startRow ||
        static_cast<int>(suggestion.start_index) > endRow) {
      continue;
    }

    if (overlapMatch < 0)
      overlapMatch = static_cast<int>(index);
  }

  return overlapMatch;
}

int MainWindow::suggestionRowForPosition(qint64 positionUs) const
{
  size_t count = toaster_suggestion_list_count(m_suggestions);

  for (size_t index = 0; index < count; ++index) {
    toaster_suggestion_t suggestion;

    if (!toaster_suggestion_list_get(m_suggestions, index, &suggestion))
      continue;
    if (positionUs >= suggestion.start_us && positionUs <= suggestion.end_us)
      return static_cast<int>(index);
  }

  return -1;
}

void MainWindow::syncPlaybackToSelection()
{
  QList<int> rows = selectedRows();
  toaster_word_t word;

  if (!m_project || rows.isEmpty())
    return;

  if (!toaster_transcript_get_word(toaster_project_get_transcript(m_project),
                                   static_cast<size_t>(rows.first()), &word)) {
    return;
  }

  m_player->setPosition(word.start_us / 1000);
}

QString MainWindow::locateTool(const QString &toolName) const
{
  QString appLocal = QCoreApplication::applicationDirPath() + "/" + toolName;
  QString mingwPath = "C:/msys64/mingw64/bin/" + toolName;
  QString pathLookup = QStandardPaths::findExecutable(toolName);

  if (QFileInfo::exists(appLocal))
    return appLocal;
  if (QFileInfo::exists(mingwPath))
    return mingwPath;
  if (!pathLookup.isEmpty())
    return pathLookup;
  return toolName;
}

bool MainWindow::runProcess(const QString &program, const QStringList &arguments, QString *stdOut,
                            QString *stdErr) const
{
  QProcess process;

  process.start(program, arguments);
  if (!process.waitForStarted(5000)) {
    if (stdErr)
      *stdErr = process.errorString();
    return false;
  }

  process.waitForFinished(-1);
  if (stdOut)
    *stdOut = QString::fromUtf8(process.readAllStandardOutput());
  if (stdErr)
    *stdErr = QString::fromUtf8(process.readAllStandardError());

  return process.exitStatus() == QProcess::NormalExit && process.exitCode() == 0;
}

bool MainWindow::runAutomationWorkflow(const QString &mediaPath, const QString &projectPath,
                                       const QString &exportPath, QString *errorMessage)
{
  toaster_project_t *reloadedProject;
  toaster_transcript_t *transcript;

  if (mediaPath.isEmpty() || projectPath.isEmpty() || exportPath.isEmpty()) {
    if (errorMessage)
      *errorMessage = "Automation workflow requires media, project, and export paths.";
    return false;
  }

  openMedia(mediaPath);
  if (!m_project) {
    if (errorMessage)
      *errorMessage = "Failed to open automation media.";
    return false;
  }

  if (!replaceTranscriptFromText("Add um Release Item", 4200000, errorMessage))
    return false;
  transcript = toaster_project_get_transcript(m_project);
  if (!transcript ||
      !toaster_transcript_set_word_times(transcript, 0, 0, 1000000) ||
      !toaster_transcript_set_word_times(transcript, 1, 1000000, 1400000) ||
      !toaster_transcript_set_word_times(transcript, 2, 1400000, 2300000) ||
      !toaster_transcript_set_word_times(transcript, 3, 3200000, 4200000) ||
      !toaster_transcript_silence_range(transcript, 3, 3)) {
    if (errorMessage)
      *errorMessage = "Failed to prepare automation transcript.";
    return false;
  }

  analyzeCleanup();
  if (toaster_suggestion_list_count(m_suggestions) < 2) {
    if (errorMessage)
      *errorMessage = "Expected filler and pause suggestions during automation.";
    return false;
  }

  applyAllSuggestions();
  if (toaster_suggestion_list_count(m_suggestions) != 0) {
    if (errorMessage)
      *errorMessage = "Suggestions remained after applying the automation workflow.";
    return false;
  }

  if (!saveProjectToPath(projectPath, errorMessage))
    return false;

  reloadedProject = toaster_project_load(m_projectPath.toUtf8().constData());
  if (!reloadedProject) {
    if (errorMessage)
      *errorMessage = QString("Failed to reload saved project:\n%1").arg(m_projectPath);
    return false;
  }

  loadProject(reloadedProject, m_projectPath);
  if (!exportMediaToPath(exportPath, errorMessage))
    return false;

  if (!QFileInfo::exists(exportPath) || QFileInfo(exportPath).size() == 0) {
    if (errorMessage)
      *errorMessage = QString("Export output was not created:\n%1").arg(exportPath);
    return false;
  }

  appendLogLine("Automation workflow complete.");
  return true;
}

bool MainWindow::runTranscriptionAutomation(const QString &mediaPath, const QString &projectPath,
                                            QString *errorMessage)
{
  toaster_transcript_t *transcript;

  openMedia(mediaPath);
  transcript = m_project ? toaster_project_get_transcript(m_project) : nullptr;

  if (transcript && toaster_transcript_word_count(transcript) == 0 &&
      m_lastTranscriptionError.isEmpty()) {
    if (!transcribeCurrentMedia(true, errorMessage))
      return false;
  }

  transcript = m_project ? toaster_project_get_transcript(m_project) : nullptr;

  if (!transcript || toaster_transcript_word_count(transcript) == 0) {
    if (errorMessage)
      *errorMessage = m_lastTranscriptionError.isEmpty()
                        ? "Transcription automation did not produce any transcript words."
                        : m_lastTranscriptionError;
    return false;
  }

  if (!QFileInfo::exists(waveformCachePath(mediaPath))) {
    if (errorMessage)
      *errorMessage = "Transcription automation did not generate a waveform cache image.";
    return false;
  }

  if (!saveProjectToPath(projectPath, errorMessage))
    return false;

  appendLogLine("Transcription automation complete.");
  return true;
}
