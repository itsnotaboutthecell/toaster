#include "WaveformView.h"

#include <QMouseEvent>
#include <QPainter>

#include <algorithm>
#include <cmath>

namespace {

void drawRangeOverlay(QPainter *painter, const QRect &drawRect, qint64 durationUs,
                      const toaster_time_range_t &range, const QColor &color)
{
  if (!painter || durationUs <= 0 || range.end_us <= range.start_us)
    return;

  auto positionFor = [durationUs, &drawRect](qint64 value) {
    double ratio = std::clamp(static_cast<double>(value) / static_cast<double>(durationUs), 0.0, 1.0);
    return drawRect.left() + static_cast<int>(ratio * drawRect.width());
  };

  int left = positionFor(range.start_us);
  int right = positionFor(range.end_us);

  if (right <= left)
    right = left + 1;

  painter->fillRect(QRect(left, drawRect.top(), right - left, drawRect.height()), color);
}

}  // namespace

WaveformView::WaveformView(QWidget *parent) : QWidget(parent)
{
  setMinimumHeight(180);
  setMouseTracking(true);
}

void WaveformView::clear()
{
  m_waveformImage = QImage();
  m_durationUs = 0;
  m_playheadUs = 0;
  m_hasSelectedRange = false;
  m_deletedRanges.clear();
  m_cutRanges.clear();
  m_silencedRanges.clear();
  m_wordRanges.clear();
  m_dragMode = DragMode::None;
  m_dragWordIndex = -1;
  update();
}

void WaveformView::setWaveformImage(const QImage &image)
{
  m_waveformImage = image;
  update();
}

void WaveformView::setDurationUs(qint64 durationUs)
{
  m_durationUs = std::max<qint64>(0, durationUs);
  update();
}

void WaveformView::setPlayheadUs(qint64 playheadUs)
{
  m_playheadUs = std::max<qint64>(0, playheadUs);
  update();
}

void WaveformView::setDeletedRanges(const QVector<toaster_time_range_t> &ranges)
{
  m_deletedRanges = ranges;
  update();
}

void WaveformView::setCutRanges(const QVector<toaster_time_range_t> &ranges)
{
  m_cutRanges = ranges;
  update();
}

void WaveformView::setSilencedRanges(const QVector<toaster_time_range_t> &ranges)
{
  m_silencedRanges = ranges;
  update();
}

void WaveformView::setSelectedRange(const toaster_time_range_t &range)
{
  m_selectedRange = range;
  m_hasSelectedRange = true;
  update();
}

void WaveformView::clearSelectedRange()
{
  m_hasSelectedRange = false;
  update();
}

void WaveformView::setWordBoundaries(const QVector<toaster_time_range_t> &wordRanges)
{
  m_wordRanges = wordRanges;
  update();
}

void WaveformView::setSnapEnabled(bool enabled)
{
  m_snapEnabled = enabled;
}

void WaveformView::setSnapGridUs(qint64 gridUs)
{
  m_snapGridUs = std::max<qint64>(0, gridUs);
}

QRect WaveformView::contentRect() const
{
  return rect().adjusted(10, 10, -10, -10);
}

int WaveformView::xForTime(qint64 positionUs, const QRect &drawRect) const
{
  if (m_durationUs <= 0)
    return drawRect.left();

  double ratio =
    std::clamp(static_cast<double>(positionUs) / static_cast<double>(m_durationUs), 0.0, 1.0);
  return drawRect.left() + static_cast<int>(ratio * drawRect.width());
}

qint64 WaveformView::timeForX(int x, const QRect &drawRect) const
{
  if (m_durationUs <= 0 || drawRect.width() <= 0)
    return 0;

  int clampedX = std::clamp(x, drawRect.left(), drawRect.right());
  double ratio = static_cast<double>(clampedX - drawRect.left()) / static_cast<double>(drawRect.width());
  return static_cast<qint64>(ratio * static_cast<double>(m_durationUs));
}

int WaveformView::hitTestBoundary(int x, const QRect &drawRect) const
{
  for (int i = 0; i < m_wordRanges.size(); ++i) {
    int startX = xForTime(m_wordRanges[i].start_us, drawRect);
    int endX = xForTime(m_wordRanges[i].end_us, drawRect);

    if (std::abs(x - startX) <= kHandleHitPx)
      return i * 2;
    if (std::abs(x - endX) <= kHandleHitPx)
      return i * 2 + 1;
  }
  return -1;
}

qint64 WaveformView::snapTime(qint64 timeUs) const
{
  if (!m_snapEnabled)
    return timeUs;

  qint64 bestTime = timeUs;
  qint64 bestDist = INT64_MAX;

  /* Snap to word boundaries */
  for (const auto &range : m_wordRanges) {
    qint64 distStart = std::abs(timeUs - range.start_us);
    qint64 distEnd = std::abs(timeUs - range.end_us);
    if (distStart < bestDist) {
      bestDist = distStart;
      bestTime = range.start_us;
    }
    if (distEnd < bestDist) {
      bestDist = distEnd;
      bestTime = range.end_us;
    }
  }

  /* Snap to time grid if set */
  if (m_snapGridUs > 0) {
    qint64 nearest = ((timeUs + m_snapGridUs / 2) / m_snapGridUs) * m_snapGridUs;
    qint64 distGrid = std::abs(timeUs - nearest);
    if (distGrid < bestDist) {
      bestDist = distGrid;
      bestTime = nearest;
    }
  }

  /* Threshold: snap only when close enough (50000 us = 50ms) */
  qint64 snapThresholdUs = 50000;
  if (bestDist < snapThresholdUs)
    return bestTime;

  return timeUs;
}

void WaveformView::updateCursorForPosition(int x)
{
  QRect drawRect = contentRect();
  int hit = hitTestBoundary(x, drawRect);

  if (hit >= 0)
    setCursor(Qt::SplitHCursor);
  else
    setCursor(Qt::ArrowCursor);
}

void WaveformView::paintEvent(QPaintEvent *event)
{
  Q_UNUSED(event);

  QPainter painter(this);
  QRect drawRect = contentRect();

  painter.fillRect(rect(), QColor(24, 26, 32));
  painter.fillRect(drawRect, QColor(16, 18, 24));

  if (!m_waveformImage.isNull()) {
    painter.drawImage(drawRect,
                      m_waveformImage.scaled(drawRect.size(), Qt::IgnoreAspectRatio,
                                             Qt::SmoothTransformation));
  } else {
    painter.setPen(QColor(190, 194, 201));
    painter.drawText(drawRect, Qt::AlignCenter, "Waveform unavailable for current media");
  }

  for (const toaster_time_range_t &range : m_deletedRanges)
    drawRangeOverlay(&painter, drawRect, m_durationUs, range, QColor(255, 157, 66, 70));

  for (const toaster_time_range_t &range : m_cutRanges)
    drawRangeOverlay(&painter, drawRect, m_durationUs, range, QColor(225, 64, 64, 95));

  for (const toaster_time_range_t &range : m_silencedRanges)
    drawRangeOverlay(&painter, drawRect, m_durationUs, range, QColor(64, 128, 255, 80));

  if (m_hasSelectedRange)
    drawRangeOverlay(&painter, drawRect, m_durationUs, m_selectedRange, QColor(255, 224, 110, 70));

  /* Draw word boundary markers */
  if (!m_wordRanges.isEmpty() && m_durationUs > 0) {
    QPen boundaryPen(QColor(120, 180, 120, 140), 1, Qt::DashLine);
    painter.setPen(boundaryPen);
    for (const auto &range : m_wordRanges) {
      int startX = xForTime(range.start_us, drawRect);
      int endX = xForTime(range.end_us, drawRect);
      painter.drawLine(startX, drawRect.top(), startX, drawRect.bottom());
      painter.drawLine(endX, drawRect.top(), endX, drawRect.bottom());
    }

    /* Draw drag handles as small triangles at top/bottom of boundaries */
    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(120, 200, 120, 200));
    for (const auto &range : m_wordRanges) {
      int startX = xForTime(range.start_us, drawRect);
      int endX = xForTime(range.end_us, drawRect);
      QPolygon topHandle;
      topHandle << QPoint(startX - 4, drawRect.top())
                << QPoint(startX + 4, drawRect.top())
                << QPoint(startX, drawRect.top() + 6);
      painter.drawPolygon(topHandle);
      QPolygon bottomHandle;
      bottomHandle << QPoint(endX - 4, drawRect.bottom())
                   << QPoint(endX + 4, drawRect.bottom())
                   << QPoint(endX, drawRect.bottom() - 6);
      painter.drawPolygon(bottomHandle);
    }
  }

  /* Draw drag preview line */
  if (m_dragMode != DragMode::None && m_durationUs > 0) {
    int dragX = xForTime(m_dragTimeUs, drawRect);
    painter.setPen(QPen(QColor(255, 200, 50), 2));
    painter.drawLine(dragX, drawRect.top(), dragX, drawRect.bottom());
  }

  if (m_durationUs > 0) {
    int playheadX = xForTime(m_playheadUs, drawRect);
    painter.setPen(QPen(QColor(255, 96, 96), 2));
    painter.drawLine(playheadX, drawRect.top(), playheadX, drawRect.bottom());
  }

  painter.setPen(QColor(72, 77, 88));
  painter.drawRect(drawRect.adjusted(0, 0, -1, -1));
}

void WaveformView::mousePressEvent(QMouseEvent *event)
{
  if (event->button() != Qt::LeftButton || m_durationUs <= 0) {
    QWidget::mousePressEvent(event);
    return;
  }

  QRect drawRect = contentRect();
  int x = static_cast<int>(event->position().x());
  int hit = hitTestBoundary(x, drawRect);

  if (hit >= 0) {
    int wordIdx = hit / 2;
    bool isEnd = (hit % 2) == 1;

    /* Check for roll: if dragging an end boundary that touches the next word's start */
    if (isEnd && wordIdx + 1 < m_wordRanges.size() &&
        m_wordRanges[wordIdx].end_us == m_wordRanges[wordIdx + 1].start_us) {
      m_dragMode = DragMode::Roll;
      m_dragWordIndex = wordIdx;
    } else if (!isEnd && wordIdx > 0 &&
               m_wordRanges[wordIdx].start_us == m_wordRanges[wordIdx - 1].end_us) {
      m_dragMode = DragMode::Roll;
      m_dragWordIndex = wordIdx - 1;
    } else if (isEnd) {
      m_dragMode = DragMode::BoundaryRight;
      m_dragWordIndex = wordIdx;
    } else {
      m_dragMode = DragMode::BoundaryLeft;
      m_dragWordIndex = wordIdx;
    }

    m_dragTimeUs = timeForX(x, drawRect);
    update();
    return;
  }

  emit seekRequested(timeForX(x, drawRect));
  QWidget::mousePressEvent(event);
}

void WaveformView::mouseMoveEvent(QMouseEvent *event)
{
  int x = static_cast<int>(event->position().x());

  if (m_dragMode != DragMode::None) {
    QRect drawRect = contentRect();
    m_dragTimeUs = snapTime(timeForX(x, drawRect));
    update();
    return;
  }

  updateCursorForPosition(x);
  QWidget::mouseMoveEvent(event);
}

void WaveformView::mouseReleaseEvent(QMouseEvent *event)
{
  if (event->button() == Qt::LeftButton && m_dragMode != DragMode::None) {
    QRect drawRect = contentRect();
    int x = static_cast<int>(event->position().x());
    qint64 finalTime = snapTime(timeForX(x, drawRect));

    if (m_dragMode == DragMode::Roll) {
      emit rollDragFinished(m_dragWordIndex, finalTime);
    } else if (m_dragWordIndex >= 0 && m_dragWordIndex < m_wordRanges.size()) {
      qint64 newStart = m_wordRanges[m_dragWordIndex].start_us;
      qint64 newEnd = m_wordRanges[m_dragWordIndex].end_us;

      if (m_dragMode == DragMode::BoundaryLeft)
        newStart = finalTime;
      else
        newEnd = finalTime;

      if (newStart < newEnd)
        emit boundaryDragFinished(m_dragWordIndex, newStart, newEnd);
    }

    m_dragMode = DragMode::None;
    m_dragWordIndex = -1;
    updateCursorForPosition(x);
    update();
  }

  QWidget::mouseReleaseEvent(event);
}
