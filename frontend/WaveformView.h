#pragma once

#include <QImage>
#include <QWidget>

#include <QVector>

extern "C" {
#include "toaster.h"
}

class WaveformView : public QWidget {
  Q_OBJECT

public:
  explicit WaveformView(QWidget *parent = nullptr);

  void clear();
  void setWaveformImage(const QImage &image);
  void setDurationUs(qint64 durationUs);
  void setPlayheadUs(qint64 playheadUs);
  void setDeletedRanges(const QVector<toaster_time_range_t> &ranges);
  void setCutRanges(const QVector<toaster_time_range_t> &ranges);
  void setSilencedRanges(const QVector<toaster_time_range_t> &ranges);
  void setSelectedRange(const toaster_time_range_t &range);
  void clearSelectedRange();
  void setWordBoundaries(const QVector<toaster_time_range_t> &wordRanges);
  void setSnapEnabled(bool enabled);
  void setSnapGridUs(qint64 gridUs);

  enum class DragMode { None, BoundaryLeft, BoundaryRight, Roll };

signals:
  void seekRequested(qint64 positionUs);
  void boundaryDragFinished(int wordIndex, qint64 newStartUs, qint64 newEndUs);
  void rollDragFinished(int leftWordIndex, qint64 newBoundaryUs);

protected:
  void paintEvent(QPaintEvent *event) override;
  void mousePressEvent(QMouseEvent *event) override;
  void mouseMoveEvent(QMouseEvent *event) override;
  void mouseReleaseEvent(QMouseEvent *event) override;

private:
  static constexpr int kHandleHitPx = 6;

  QRect contentRect() const;
  int xForTime(qint64 positionUs, const QRect &drawRect) const;
  qint64 timeForX(int x, const QRect &drawRect) const;
  int hitTestBoundary(int x, const QRect &drawRect) const;
  qint64 snapTime(qint64 timeUs) const;
  void updateCursorForPosition(int x);

  QImage m_waveformImage;
  qint64 m_durationUs = 0;
  qint64 m_playheadUs = 0;
  bool m_hasSelectedRange = false;
  toaster_time_range_t m_selectedRange{};
  QVector<toaster_time_range_t> m_deletedRanges;
  QVector<toaster_time_range_t> m_cutRanges;
  QVector<toaster_time_range_t> m_silencedRanges;
  QVector<toaster_time_range_t> m_wordRanges;

  DragMode m_dragMode = DragMode::None;
  int m_dragWordIndex = -1;
  qint64 m_dragTimeUs = 0;
  bool m_snapEnabled = true;
  qint64 m_snapGridUs = 0;
};
