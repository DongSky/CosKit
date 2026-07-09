/**
 * MaskEditor — 4-canvas layered mask drawing tool.
 * Canvas layers (bottom to top):
 *   1. imageCanvas — displays the source image
 *   2. previewCanvas — blue semi-transparent overlay on edit regions
 *   3. maskCanvas — actual mask data (hidden, opacity:0)
 *   4. cursorCanvas — custom brush cursor (pointer-events:none)
 *
 * Mask convention: white opaque (alpha=255) = protect, transparent (alpha=0) = edit.
 */

class MaskEditor {
  constructor() {
    this.modal = document.getElementById('mask-editor-modal');
    this.stack = document.getElementById('mask-canvas-stack');
    this.imageCanvas = document.getElementById('mask-layer-image');
    this.previewCanvas = document.getElementById('mask-layer-preview');
    this.maskCanvas = document.getElementById('mask-layer-draw');
    this.cursorCanvas = document.getElementById('mask-layer-cursor');

    this.tool = 'brush'; // brush | eraser | rect | polygon
    this.brushSize = 30;
    this.featherRadius = 0;
    this.undoStack = []; // PNG data URL snapshots (compressed, not raw ImageData)
    this.redoStack = [];
    this.maxUndo = 40;

    // Polygon state
    this.polygonPoints = [];
    this._polyCursor = null; // last cursor position, drives the rubber-band preview

    // Rect drag state
    this._rectStart = null;
    this._drawing = false;

    // Working dimensions
    this._workW = 0;
    this._workH = 0;

    this._bindEvents();
  }

  _bindEvents() {
    // Toolbar
    document.querySelectorAll('.mask-tool').forEach(btn => {
      btn.addEventListener('click', () => {
        document.querySelectorAll('.mask-tool').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        this._finishPolygon(); // commit pending polygon before switching
        this.tool = btn.dataset.tool;
        this._updatePolyCloseVisibility();
      });
    });

    document.getElementById('mask-brush-size').addEventListener('input', e => {
      this.brushSize = parseInt(e.target.value, 10);
    });
    document.getElementById('mask-feather').addEventListener('input', e => {
      this.featherRadius = parseInt(e.target.value, 10);
    });
    document.getElementById('mask-undo').addEventListener('click', () => this.undo());
    document.getElementById('mask-redo').addEventListener('click', () => this.redo());
    document.getElementById('mask-clear').addEventListener('click', () => this.clear());
    document.getElementById('mask-invert').addEventListener('click', () => this.invert());
    document.getElementById('mask-poly-close').addEventListener('click', () => this._finishPolygon());
    document.getElementById('mask-remove').addEventListener('click', () => this.removeSelection());
    document.getElementById('mask-cancel').addEventListener('click', () => this.close());
    document.getElementById('mask-confirm').addEventListener('click', () => this.confirm());

    // Keyboard: Esc cancels the in-progress polygon first, then closes the
    // modal; Enter commits the pending polygon; Backspace removes the last
    // vertex (misclick recovery).
    document.addEventListener('keydown', e => {
      if (this.modal.style.display === 'none' || !this.modal.style.display) return;
      if (e.key === 'Escape') {
        if (this.polygonPoints.length > 0) {
          this.polygonPoints = [];
          this._polyCursor = null;
          this._clearCursorCanvas();
        } else {
          this.close();
        }
        return;
      }
      if (this.tool === 'polygon' && this.polygonPoints.length > 0) {
        if (e.key === 'Enter') {
          e.preventDefault();
          this._finishPolygon();
        } else if (e.key === 'Backspace') {
          e.preventDefault();
          this.polygonPoints.pop();
          this._renderPolygonPreview();
        }
      }
    });

    // Drawing events on cursorCanvas (top layer, receives pointer events)
    const c = this.cursorCanvas;
    c.addEventListener('pointerdown', e => this._onPointerDown(e));
    c.addEventListener('pointermove', e => this._onPointerMove(e));
    c.addEventListener('pointerup', e => this._onPointerUp(e));
    c.addEventListener('pointerleave', e => this._onPointerUp(e));
    c.addEventListener('dblclick', e => this._onDoubleClick(e));
    c.addEventListener('contextmenu', e => {
      e.preventDefault();
      // Right-click: remove the last polygon vertex (misclick recovery)
      if (this.tool === 'polygon' && this.polygonPoints.length > 0) {
        this.polygonPoints.pop();
        this._renderPolygonPreview();
      }
    });
  }

  /**
   * Open the editor for an image. Pass existingMaskDataUrl to restore a
   * previously confirmed selection (must have been drawn on the same image).
   */
  open(imageDataUrl, imageWidth, imageHeight, existingMaskDataUrl) {
    const { w, h } = this._computeWorkingSize(imageWidth, imageHeight);
    this._workW = w;
    this._workH = h;

    [this.imageCanvas, this.previewCanvas, this.maskCanvas, this.cursorCanvas].forEach(canvas => {
      canvas.width = w;
      canvas.height = h;
    });

    // Draw source image
    const img = new Image();
    img.onload = () => {
      const ctx = this.imageCanvas.getContext('2d');
      ctx.drawImage(img, 0, 0, w, h);
    };
    img.src = imageDataUrl;

    // Fill mask white (all protected)
    const maskCtx = this.maskCanvas.getContext('2d');
    maskCtx.fillStyle = '#ffffff';
    maskCtx.fillRect(0, 0, w, h);

    // Restore existing mask if provided (async — re-render preview when loaded)
    if (existingMaskDataUrl) {
      const maskImg = new Image();
      maskImg.onload = () => {
        maskCtx.clearRect(0, 0, w, h);
        maskCtx.drawImage(maskImg, 0, 0, w, h);
        this.renderPreview();
      };
      maskImg.src = existingMaskDataUrl;
    }

    this.undoStack = [];
    this.redoStack = [];
    this.polygonPoints = [];
    this._rectStart = null;
    this._drawing = false;

    this.renderPreview();
    this._updatePolyCloseVisibility();

    // Fit canvas stack to viewport
    const maxViewW = window.innerWidth * 0.85;
    const maxViewH = window.innerHeight * 0.72;
    const scale = Math.min(1, maxViewW / w, maxViewH / h);
    this.stack.style.width = (w * scale) + 'px';
    this.stack.style.height = (h * scale) + 'px';

    this.modal.style.display = 'flex';
    this._onConfirm = null;
  }

  close() {
    this.modal.style.display = 'none';
    this.polygonPoints = [];
    this._polyCursor = null;
    this._rectStart = null;
    this._drawing = false;
    this._clearCursorCanvas();
  }

  confirm() {
    // Classify selection coverage before accepting
    const coverage = this._classifyCoverage();
    if (coverage === 'empty') {
      alert('请先绘制选区（画笔涂抹、拖拽矩形或点击多边形顶点）');
      return;
    }
    if (coverage === 'full') {
      if (!window.confirm('选区覆盖了整张图片，效果等同于全图编辑。确定继续吗？')) {
        return;
      }
    }
    const dataUrl = this.exportMaskAsDataUrl();
    this.close();
    if (this._onConfirm) this._onConfirm(dataUrl);
  }

  /** Remove the current selection entirely and close. */
  removeSelection() {
    this.close();
    if (this._onConfirm) this._onConfirm(null);
  }

  onConfirm(callback) {
    this._onConfirm = callback;
  }

  // --- Drawing logic ---

  _canvasXY(e) {
    // Compute scale from the live bounding rect so CSS/layout changes can't
    // desync coordinates.
    const rect = this.cursorCanvas.getBoundingClientRect();
    const scaleX = rect.width > 0 ? this.cursorCanvas.width / rect.width : 1;
    const scaleY = rect.height > 0 ? this.cursorCanvas.height / rect.height : 1;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: (e.clientY - rect.top) * scaleY
    };
  }

  // Snapshots are stored as PNG data URLs (a few KB each) instead of raw
  // ImageData (~14 MB at 1920px) to keep undo memory bounded.
  _snapshot() {
    return this.maskCanvas.toDataURL('image/png');
  }

  _restoreSnapshot(snapshot) {
    const img = new Image();
    img.onload = () => {
      const ctx = this.maskCanvas.getContext('2d');
      ctx.clearRect(0, 0, this._workW, this._workH);
      ctx.drawImage(img, 0, 0);
      this.renderPreview();
    };
    img.src = snapshot;
  }

  _pushUndo() {
    this.undoStack.push(this._snapshot());
    if (this.undoStack.length > this.maxUndo) this.undoStack.shift();
    this.redoStack = [];
  }

  undo() {
    if (this.undoStack.length === 0) return;
    this.redoStack.push(this._snapshot());
    this._restoreSnapshot(this.undoStack.pop());
  }

  redo() {
    if (this.redoStack.length === 0) return;
    this.undoStack.push(this._snapshot());
    this._restoreSnapshot(this.redoStack.pop());
  }

  clear() {
    this._pushUndo();
    const ctx = this.maskCanvas.getContext('2d');
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, this._workW, this._workH);
    this.polygonPoints = [];
    this._clearCursorCanvas();
    this.renderPreview();
  }

  invert() {
    this._pushUndo();
    const ctx = this.maskCanvas.getContext('2d');
    const data = ctx.getImageData(0, 0, this._workW, this._workH);
    for (let i = 3; i < data.data.length; i += 4) {
      data.data[i] = 255 - data.data[i];
    }
    ctx.putImageData(data, 0, 0);
    this.renderPreview();
  }

  _onPointerDown(e) {
    if (e.button !== undefined && e.button !== 0) return; // right/middle button never draws
    if (this.tool === 'polygon') {
      const { x, y } = this._canvasXY(e);
      // Clicking back near the first vertex closes the polygon — snap radius
      // means it doesn't require pixel-precise aim.
      if (this.polygonPoints.length >= 3) {
        const first = this.polygonPoints[0];
        if (Math.hypot(x - first.x, y - first.y) <= this._polySnapRadius()) {
          this._finishPolygon();
          return;
        }
      }
      this.polygonPoints.push({ x, y });
      this._renderPolygonPreview(x, y);
      return;
    }

    this._drawing = true;
    this._pushUndo();
    const { x, y } = this._canvasXY(e);
    this._lastPoint = { x, y };

    if (this.tool === 'rect') {
      this._rectStart = { x, y };
      return;
    }

    // Brush or eraser: draw initial dot
    this._drawStrokePoint(x, y);
    this.renderPreview();
  }

  _onPointerMove(e) {
    const { x, y } = this._canvasXY(e);

    if (this.tool === 'polygon') {
      this._polyCursor = { x, y };
      this._renderPolygonPreview(x, y);
      return;
    }

    this._renderCursor(x, y);

    if (!this._drawing) return;

    if (this.tool === 'rect') {
      this._renderRectPreview(x, y);
      return;
    }

    if (this.tool === 'brush' || this.tool === 'eraser') {
      this._drawStrokeLine(this._lastPoint.x, this._lastPoint.y, x, y);
      this._lastPoint = { x, y };
      this.renderPreview();
    }
  }

  _onPointerUp(e) {
    if (!this._drawing) return;
    this._drawing = false;

    if (this.tool === 'rect' && this._rectStart) {
      const { x, y } = this._canvasXY(e);
      this._applyRect(this._rectStart.x, this._rectStart.y, x, y);
      this._rectStart = null;
      this._clearCursorCanvas();
      this.renderPreview();
    }
  }

  _onDoubleClick(e) {
    if (this.tool === 'polygon') this._finishPolygon();
  }

  _finishPolygon() {
    // Filter out consecutive near-duplicate vertices (dblclick fires two
    // pointerdowns at nearly the same position).
    const pts = [];
    for (const p of this.polygonPoints) {
      const last = pts[pts.length - 1];
      if (!last || Math.hypot(p.x - last.x, p.y - last.y) > 2) pts.push(p);
    }
    if (pts.length >= 3) {
      this._pushUndo();
      this._applyPolygon(pts);
      this.renderPreview();
    }
    this.polygonPoints = [];
    this._polyCursor = null;
    this._clearCursorCanvas();
  }

  _updatePolyCloseVisibility() {
    const btn = document.getElementById('mask-poly-close');
    btn.style.display = this.tool === 'polygon' ? '' : 'none';
  }

  // --- Stroke rendering ---

  _drawStrokePoint(x, y) {
    const ctx = this.maskCanvas.getContext('2d');
    ctx.globalCompositeOperation = this.tool === 'eraser' ? 'source-over' : 'destination-out';
    ctx.fillStyle = this.tool === 'eraser' ? '#ffffff' : '#000000';
    ctx.beginPath();
    ctx.arc(x, y, this.brushSize / 2, 0, Math.PI * 2);
    ctx.fill();
    ctx.globalCompositeOperation = 'source-over';
  }

  _drawStrokeLine(x1, y1, x2, y2) {
    const ctx = this.maskCanvas.getContext('2d');
    ctx.globalCompositeOperation = this.tool === 'eraser' ? 'source-over' : 'destination-out';
    ctx.strokeStyle = this.tool === 'eraser' ? '#ffffff' : '#000000';
    ctx.lineWidth = this.brushSize;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();
    ctx.globalCompositeOperation = 'source-over';
  }

  _applyRect(x1, y1, x2, y2) {
    const ctx = this.maskCanvas.getContext('2d');
    ctx.globalCompositeOperation = 'destination-out';
    const rx = Math.min(x1, x2), ry = Math.min(y1, y2);
    const rw = Math.abs(x2 - x1), rh = Math.abs(y2 - y1);
    ctx.fillRect(rx, ry, rw, rh);
    ctx.globalCompositeOperation = 'source-over';
  }

  _applyPolygon(points) {
    const ctx = this.maskCanvas.getContext('2d');
    ctx.globalCompositeOperation = 'destination-out';
    ctx.beginPath();
    ctx.moveTo(points[0].x, points[0].y);
    for (let i = 1; i < points.length; i++) {
      ctx.lineTo(points[i].x, points[i].y);
    }
    ctx.closePath();
    ctx.fill();
    ctx.globalCompositeOperation = 'source-over';
  }

  // --- Preview rendering ---

  renderPreview() {
    const ctx = this.previewCanvas.getContext('2d');
    const w = this._workW, h = this._workH;
    ctx.clearRect(0, 0, w, h);
    // Fill with blue tint
    ctx.fillStyle = 'rgba(59, 130, 246, 0.5)';
    ctx.fillRect(0, 0, w, h);
    // Cut out protected areas (white/opaque in mask)
    ctx.globalCompositeOperation = 'destination-out';
    ctx.drawImage(this.maskCanvas, 0, 0);
    ctx.globalCompositeOperation = 'source-over';
  }

  _renderRectPreview(x, y) {
    const ctx = this.cursorCanvas.getContext('2d');
    ctx.clearRect(0, 0, this._workW, this._workH);
    if (!this._rectStart) return;
    const rx = Math.min(this._rectStart.x, x), ry = Math.min(this._rectStart.y, y);
    const rw = Math.abs(x - this._rectStart.x), rh = Math.abs(y - this._rectStart.y);
    ctx.strokeStyle = 'rgba(255,255,255,0.8)';
    ctx.lineWidth = 2;
    ctx.setLineDash([6, 4]);
    ctx.strokeRect(rx, ry, rw, rh);
    ctx.setLineDash([]);
  }

  /**
   * Polygon preview: committed edges, rubber-band from the last vertex to the
   * cursor, a faint closing edge back to the first vertex, and a green
   * snap-highlight on the first vertex when the cursor is close enough that a
   * click would close the polygon.
   */
  _renderPolygonPreview(cx, cy) {
    if (cx === undefined && this._polyCursor) {
      cx = this._polyCursor.x;
      cy = this._polyCursor.y;
    }
    const ctx = this.cursorCanvas.getContext('2d');
    ctx.clearRect(0, 0, this._workW, this._workH);
    const pts = this.polygonPoints;
    if (pts.length < 1) return;

    const snapR = this._polySnapRadius();
    const hoverClose = pts.length >= 3 && cx !== undefined &&
      Math.hypot(cx - pts[0].x, cy - pts[0].y) <= snapR;

    // Committed edges
    ctx.strokeStyle = 'rgba(255,255,255,0.8)';
    ctx.lineWidth = 2;
    ctx.setLineDash([6, 4]);
    ctx.beginPath();
    ctx.moveTo(pts[0].x, pts[0].y);
    for (let i = 1; i < pts.length; i++) {
      ctx.lineTo(pts[i].x, pts[i].y);
    }
    ctx.stroke();

    // Rubber band: last vertex → cursor, plus a fainter closing edge back to
    // the first vertex so the final shape is visible before committing.
    if (cx !== undefined) {
      const last = pts[pts.length - 1];
      ctx.strokeStyle = hoverClose ? 'rgba(34,197,94,0.9)' : 'rgba(255,255,255,0.5)';
      ctx.beginPath();
      ctx.moveTo(last.x, last.y);
      ctx.lineTo(cx, cy);
      ctx.stroke();
      if (pts.length >= 2 && !hoverClose) {
        ctx.strokeStyle = 'rgba(255,255,255,0.25)';
        ctx.beginPath();
        ctx.moveTo(cx, cy);
        ctx.lineTo(pts[0].x, pts[0].y);
        ctx.stroke();
      }
    }
    ctx.setLineDash([]);

    // Vertices — the first one is the close target: green and larger once the
    // polygon can be closed, with a ring when the cursor is in snap range.
    pts.forEach((p, i) => {
      ctx.beginPath();
      if (i === 0 && pts.length >= 3) {
        ctx.fillStyle = 'rgba(34,197,94,0.95)';
        ctx.arc(p.x, p.y, hoverClose ? 7 : 5, 0, Math.PI * 2);
      } else {
        ctx.fillStyle = 'rgba(255,255,255,0.9)';
        ctx.arc(p.x, p.y, 4, 0, Math.PI * 2);
      }
      ctx.fill();
    });
    if (hoverClose) {
      ctx.strokeStyle = 'rgba(34,197,94,0.9)';
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(pts[0].x, pts[0].y, snapR, 0, Math.PI * 2);
      ctx.stroke();
    }

    // Crosshair at the cursor for precise vertex placement
    if (cx !== undefined && !hoverClose) {
      ctx.strokeStyle = 'rgba(255,255,255,0.8)';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(cx - 5, cy); ctx.lineTo(cx + 5, cy);
      ctx.moveTo(cx, cy - 5); ctx.lineTo(cx, cy + 5);
      ctx.stroke();
    }
  }

  /**
   * Snap radius for closing the polygon: ~12 on-screen pixels converted to
   * canvas units, so the hit target stays constant regardless of how far the
   * working canvas is scaled down to fit the viewport.
   */
  _polySnapRadius() {
    const rect = this.cursorCanvas.getBoundingClientRect();
    const scale = rect.width > 0 ? this.cursorCanvas.width / rect.width : 1;
    return Math.max(8, 12 * scale);
  }

  _renderCursor(x, y) {
    if (this.tool === 'rect' && this._drawing) return;
    if (this.tool === 'polygon') return;
    const ctx = this.cursorCanvas.getContext('2d');
    ctx.clearRect(0, 0, this._workW, this._workH);
    ctx.strokeStyle = 'rgba(255,255,255,0.8)';
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.arc(x, y, this.brushSize / 2, 0, Math.PI * 2);
    ctx.stroke();
    // Crosshair
    ctx.beginPath();
    ctx.moveTo(x - 4, y); ctx.lineTo(x + 4, y);
    ctx.moveTo(x, y - 4); ctx.lineTo(x, y + 4);
    ctx.stroke();
  }

  _clearCursorCanvas() {
    const ctx = this.cursorCanvas.getContext('2d');
    ctx.clearRect(0, 0, this._workW, this._workH);
  }

  // --- Export ---

  /** empty = nothing painted; full = whole image selected; partial otherwise. */
  _classifyCoverage() {
    const ctx = this.maskCanvas.getContext('2d');
    const data = ctx.getImageData(0, 0, this._workW, this._workH).data;
    let editCount = 0;
    const total = this._workW * this._workH;
    for (let i = 3; i < data.length; i += 4) {
      if (data[i] < 128) editCount++;
    }
    if (editCount === 0) return 'empty';
    if (editCount >= total * 0.999) return 'full';
    return 'partial';
  }

  /**
   * Export the mask as a PNG data URL. Feather is applied to an offscreen
   * copy so repeated exports never compound the blur on the live canvas.
   */
  exportMaskAsDataUrl() {
    if (this.featherRadius <= 0) {
      return this.maskCanvas.toDataURL('image/png');
    }
    const off = document.createElement('canvas');
    off.width = this._workW;
    off.height = this._workH;
    const offCtx = off.getContext('2d');
    offCtx.drawImage(this.maskCanvas, 0, 0);
    this._applyFeatherTo(offCtx);
    return off.toDataURL('image/png');
  }

  _applyFeatherTo(ctx) {
    const data = ctx.getImageData(0, 0, this._workW, this._workH);
    const r = this.featherRadius;
    // Simple box blur on alpha channel
    const w = this._workW, h = this._workH;
    const alpha = new Float32Array(w * h);
    for (let i = 0; i < alpha.length; i++) alpha[i] = data.data[i * 4 + 3];
    const blurred = new Float32Array(w * h);
    // Horizontal pass
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        let sum = 0, count = 0;
        for (let dx = -r; dx <= r; dx++) {
          const nx = x + dx;
          if (nx >= 0 && nx < w) { sum += alpha[y * w + nx]; count++; }
        }
        blurred[y * w + x] = sum / count;
      }
    }
    // Vertical pass
    for (let x = 0; x < w; x++) {
      for (let y = 0; y < h; y++) {
        let sum = 0, count = 0;
        for (let dy = -r; dy <= r; dy++) {
          const ny = y + dy;
          if (ny >= 0 && ny < h) { sum += blurred[ny * w + x]; count++; }
        }
        data.data[(y * w + x) * 4 + 3] = Math.round(sum / count);
      }
    }
    ctx.putImageData(data, 0, 0);
  }

  _computeWorkingSize(w, h) {
    const MAX = 1920, MULT = 16;
    const scale = Math.min(1, MAX / Math.max(w, h));
    return {
      w: Math.floor(w * scale / MULT) * MULT,
      h: Math.floor(h * scale / MULT) * MULT
    };
  }
}

// Singleton
window.maskEditor = new MaskEditor();