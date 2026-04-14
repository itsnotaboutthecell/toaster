#pragma once

#include <QMainWindow>

#include <QList>
#include <QString>

extern "C" {
#include "toaster.h"
}

class QAction;
class QAudioOutput;
class QCloseEvent;
class QDockWidget;
class QLabel;
class QLineEdit;
class QListWidget;
class QMediaPlayer;
class QPlainTextEdit;
class QPushButton;
class QSlider;
class QTableWidget;
class QToolButton;
class QVideoWidget;
class WaveformView;

class MainWindow : public QMainWindow {
public:
  explicit MainWindow(QWidget *parent = nullptr);
  ~MainWindow() override;
  bool runAutomationWorkflow(const QString &mediaPath, const QString &projectPath,
                             const QString &exportPath, QString *errorMessage = nullptr);
  bool runTranscriptionAutomation(const QString &mediaPath, const QString &projectPath,
                                  QString *errorMessage = nullptr);

protected:
  void closeEvent(QCloseEvent *event) override;

private:
  void createCentralWorkspace();
  void createDocks();
  void createMenus();
  void wirePlayer();
  void restoreWindowState();
  void saveWindowState();
  void setDocksLocked(bool locked);
  void resetDocks();
  void appendLogLine(const QString &line);

  void newProject();
  void openMedia(const QString &path);
  void openMediaDialog();
  void openProjectDialog();
  void saveProject();
  void saveProjectAs();
  void exportMedia();
  void exportCaptions();
  void exportScript();
  void transcribeMedia();
  void importTranscript();
  void analyzeCleanup();
  void applySelectedSuggestion();
  void applyAllSuggestions();
  void deleteSelection();
  void restoreSelection();
  void silenceSelection();
  void unsilenceSelection();
  void focusTranscriptSearch();
  void focusTranscriptReplace();
  void findNextTranscriptMatch();
  void findPreviousTranscriptMatch();
  void replaceCurrentMatch();
  void replaceAllMatches();
  void undoEdit();
  void redoEdit();
  void splitWordAtPlayhead();
  void updateUndoRedoState();
  void onTranscriptSearchChanged(const QString &text);

  void onTranscriptCellChanged(int row, int column);
  void onTranscriptSelectionChanged();
  void onSuggestionActivated();
  void onSuggestionSelectionChanged();
  void onDurationChanged(qint64 duration_ms);
  void onPositionChanged(qint64 position_ms);
  void onSliderMoved(int value);
  void onWaveformSeekRequested(qint64 positionUs);

  bool ensureProject();
  void clearProject();
  void loadProject(toaster_project_t *project, const QString &projectPath);
  bool saveProjectToPath(const QString &path, QString *errorMessage = nullptr);
  bool exportMediaToPath(const QString &outputPath, QString *errorMessage = nullptr);
  bool exportCaptionsToPath(const QString &outputPath, toaster_caption_format_t format,
                            QString *errorMessage = nullptr);
  bool exportScriptToPath(const QString &outputPath, QString *errorMessage = nullptr);
  bool transcribeCurrentMedia(bool allowModelDownload, QString *errorMessage = nullptr);
  bool populateTranscriptFromWhisperJson(const QString &jsonPath, QString *errorMessage = nullptr);
  bool replaceTranscriptFromText(const QString &text, qint64 duration_us,
                                 QString *errorMessage = nullptr);
  bool ensureWhisperModel(QString *modelPath, bool allowDownload,
                          QString *errorMessage = nullptr);
  bool downloadWhisperModel(const QString &modelPath, QString *errorMessage = nullptr);
  bool loadWaveformForMedia(const QString &mediaPath, QString *errorMessage = nullptr);
  bool selectTranscriptSearchMatch(int matchIndex);
  bool confirmTranscriptReplacement(const QString &title, const QString &message);
  void rebuildAllViews();
  void rebuildTranscriptTable();
  void rebuildWaveformView();
  void rebuildSuggestionList();
  void syncWaveformSelectionFromContext();
  void setTranscriptSelectionRange(int startRow, int endRow, bool seekPlayback);
  void setCurrentSuggestionRow(int row);
  void syncSuggestionSelectionForTranscriptSelection();
  void syncSuggestionSelectionForPosition(qint64 positionUs);
  void updateActiveTranscriptRow(int row);
  void updateTranscriptRowVisualState(int row);
  void scrollTranscriptRowIntoView(int row);
  void updateInspector();
  void updateTranscriptToolState(size_t wordCount, const QString &mediaPath);
  void clearTranscriptSearch();
  void refreshTranscriptSearch(bool preserveCurrentMatch);
  void syncPlaybackToSelection();
  QList<int> selectedRows() const;
  int transcriptRowForPosition(qint64 positionUs) const;
  int suggestionRowForTranscriptRange(int startRow, int endRow) const;
  int suggestionRowForPosition(qint64 positionUs) const;
  bool applySuggestion(size_t suggestionIndex);
  QString locateTool(const QString &toolName) const;
  QString locateWhisperModel() const;
  QString waveformCachePath(const QString &mediaPath) const;
  bool runProcess(const QString &program, const QStringList &arguments, QString *stdOut,
                  QString *stdErr) const;

  struct TranscriptSearchMatch {
    int startRow = -1;
    int endRow = -1;
  };

  QDockWidget *m_transcriptDock = nullptr;
  QDockWidget *m_waveformDock = nullptr;
  QDockWidget *m_suggestedEditsDock = nullptr;
  QDockWidget *m_inspectorDock = nullptr;
  QDockWidget *m_exportDock = nullptr;
  QDockWidget *m_logsDock = nullptr;
  QList<QDockWidget *> m_docks;

  QAction *m_lockDocksAction = nullptr;
  QAction *m_alwaysOnTopAction = nullptr;
  QAction *m_saveProjectAction = nullptr;
  QAction *m_exportMediaAction = nullptr;
  QAction *m_exportCaptionsAction = nullptr;
  QAction *m_exportScriptAction = nullptr;
  QAction *m_analyzeAction = nullptr;
  QAction *m_transcribeAction = nullptr;
  QAction *m_importTranscriptAction = nullptr;
  QAction *m_focusTranscriptSearchAction = nullptr;
  QAction *m_findNextAction = nullptr;
  QAction *m_findPreviousAction = nullptr;
  QAction *m_findAndReplaceAction = nullptr;
  QAction *m_undoAction = nullptr;
  QAction *m_redoAction = nullptr;

  toaster_project_t *m_project = nullptr;
  toaster_suggestion_list_t *m_suggestions = nullptr;
  QString m_projectPath;
  QString m_lastTranscriptionError;
  qint64 m_mediaDurationUs = 0;
  bool m_updatingTranscriptTable = false;
  bool m_updatingSlider = false;
  bool m_transcriptionInProgress = false;
  int m_activeTranscriptRow = -1;

  QMediaPlayer *m_player = nullptr;
  QAudioOutput *m_audioOutput = nullptr;
  QVideoWidget *m_videoWidget = nullptr;
  QSlider *m_positionSlider = nullptr;
  QPushButton *m_playButton = nullptr;

  QTableWidget *m_transcriptTable = nullptr;
  QLineEdit *m_transcriptSearchEdit = nullptr;
  QLineEdit *m_transcriptReplaceEdit = nullptr;
  QLabel *m_transcriptSearchStatusLabel = nullptr;
  QToolButton *m_transcriptTranscribeButton = nullptr;
  QToolButton *m_transcriptImportButton = nullptr;
  QToolButton *m_transcriptSearchPreviousButton = nullptr;
  QToolButton *m_transcriptSearchNextButton = nullptr;
  QToolButton *m_transcriptReplaceButton = nullptr;
  QToolButton *m_transcriptReplaceAllButton = nullptr;
  QToolButton *m_exportMediaButton = nullptr;
  QToolButton *m_exportCaptionsButton = nullptr;
  QToolButton *m_exportScriptButton = nullptr;
  WaveformView *m_waveformView = nullptr;
  QListWidget *m_suggestionList = nullptr;
  QPlainTextEdit *m_logView = nullptr;

  QLabel *m_projectLabel = nullptr;
  QLabel *m_mediaLabel = nullptr;
  QLabel *m_durationLabel = nullptr;
  QLabel *m_languageLabel = nullptr;
  QLabel *m_statsLabel = nullptr;
  QList<TranscriptSearchMatch> m_transcriptSearchMatches;
  int m_transcriptSearchIndex = -1;
};
