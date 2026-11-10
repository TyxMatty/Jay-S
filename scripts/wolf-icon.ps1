# Builds Jay-S's icon assets from its wolf line art (frontend/src-tauri/app-icon-lineart.png, one
# color on transparent, 1024x1024). The thin lines vanish at 16-32 px, so the icon is the "filled
# head": lines thickened and the inside of the head filled (chosen on 05/11/2026).
#   frontend/src/assets/brand/wolf-lines.png  white, alpha = thickened lines (512 px)
#   frontend/src/assets/brand/wolf-fill.png   white, alpha = inside of the head (512 px)
#   frontend/src/assets/brand/wolf-lineart.png white, alpha = the original thin lines (512 px)
#   frontend/src-tauri/app-icon.png           the .exe/installer icon (Suave colors, 1024 px)
# The app tints the two masks with the style's colors (CSS mask / canvas). The script is Jay-C's
# (scripts/wolf-icon.ps1 there), without its Discord pictures. After running this:
#   cd frontend; npx tauri icon src-tauri/app-icon.png   (then delete icons/ios and icons/android:
#   Jay-S is Windows only; and change build.rs's date, or the .exe keeps the old icon)
#   powershell -File scripts/wolf-icon.ps1
$root = Split-Path $PSScriptRoot -Parent
Add-Type -ReferencedAssemblies System.Drawing @'
using System; using System.Drawing; using System.Drawing.Drawing2D; using System.Drawing.Imaging;
using System.Runtime.InteropServices; using System.Collections.Generic;
public static class WolfIcon {
  const int LineGrow = 9;   // px at 1024: thick enough to read at 16 px
  const int SealGrow = 4;   // closes small gaps in the outline before finding the inside
  static byte[] Alpha(Bitmap b) {
    var d = b.LockBits(new Rectangle(0, 0, b.Width, b.Height), ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
    var px = new byte[d.Stride * b.Height]; Marshal.Copy(d.Scan0, px, 0, px.Length); b.UnlockBits(d);
    var a = new byte[b.Width * b.Height]; for (int i = 0; i < a.Length; i++) a[i] = px[i * 4 + 3]; return a;
  }
  /// Grows the lines by r pixels with a round brush (a disc), so line ends stay round.
  static byte[] Dilate(byte[] a, int w, int h, int r) {
    var offs = new List<int[]>();
    for (int dy = -r; dy <= r; dy++) for (int dx = -r; dx <= r; dx++) if (dx * dx + dy * dy <= r * r) offs.Add(new[] { dx, dy });
    var o = new byte[a.Length];
    for (int y = 0; y < h; y++) for (int x = 0; x < w; x++) {
      byte m = 0;
      foreach (var d in offs) { int xx = x + d[0], yy = y + d[1]; if (xx >= 0 && xx < w && yy >= 0 && yy < h) { byte v = a[yy * w + xx]; if (v > m) { m = v; if (m == 255) break; } } }
      o[y * w + x] = m;
    }
    return o;
  }
  /// Alpha 255 where a flood fill from the border can't reach without crossing a line.
  static byte[] Inside(byte[] lines, int w, int h) {
    var outside = new bool[w * h]; var q = new Queue<int>();
    for (int x = 0; x < w; x++) { q.Enqueue(x); q.Enqueue((h - 1) * w + x); }
    for (int y = 0; y < h; y++) { q.Enqueue(y * w); q.Enqueue(y * w + w - 1); }
    while (q.Count > 0) { int i = q.Dequeue(); if (i < 0 || i >= w * h || outside[i] || lines[i] > 60) continue; outside[i] = true; int x = i % w;
      if (x > 0) q.Enqueue(i - 1); if (x < w - 1) q.Enqueue(i + 1); q.Enqueue(i - w); q.Enqueue(i + w); }
    var o = new byte[w * h]; for (int i = 0; i < o.Length; i++) o[i] = (byte)(!outside[i] ? 255 : 0); return o;
  }
  static Bitmap Tinted(byte[] alpha, int w, int h, Color c) {
    var b = new Bitmap(w, h, PixelFormat.Format32bppArgb);
    var d = b.LockBits(new Rectangle(0, 0, w, h), ImageLockMode.WriteOnly, PixelFormat.Format32bppArgb);
    var px = new byte[d.Stride * h];
    for (int i = 0; i < w * h; i++) { px[i * 4] = c.B; px[i * 4 + 1] = c.G; px[i * 4 + 2] = c.R; px[i * 4 + 3] = alpha[i]; }
    Marshal.Copy(px, 0, d.Scan0, px.Length); b.UnlockBits(d); return b;
  }
  static void SaveScaled(Bitmap src, string path, int size, int margin) {
    using (var o = new Bitmap(size, size, PixelFormat.Format32bppArgb)) {
      using (var g = Graphics.FromImage(o)) {
        g.InterpolationMode = InterpolationMode.HighQualityBicubic; g.PixelOffsetMode = PixelOffsetMode.HighQuality;
        g.DrawImage(src, margin, margin, size - 2 * margin, size - 2 * margin);
      }
      o.Save(path, ImageFormat.Png);
    }
  }
  /// The square around the drawing (its bounding box, centered), so the wolf fills the icon.
  static byte[] CropSquare(byte[] a, int w, int h, int x0, int y0, int side) {
    var o = new byte[side * side];
    for (int y = 0; y < side; y++) for (int x = 0; x < side; x++) {
      int sx = x0 + x, sy = y0 + y;
      if (sx >= 0 && sx < w && sy >= 0 && sy < h) o[y * side + x] = a[sy * w + sx];
    }
    return o;
  }

  public static string Run(string src, string linesOut, string fillOut, string lineArtOut, string appIconOut) {
    int w, h; byte[] alpha;
    using (var s = new Bitmap(src)) { w = s.Width; h = s.Height; alpha = Alpha(s); }
    var lines = Dilate(alpha, w, h, LineGrow);
    var fill = Inside(Dilate(alpha, w, h, SealGrow), w, h);
    // Crop to the drawing: a centered square around its bounding box, plus a 1% edge for smoothing.
    int bx0 = w, by0 = h, bx1 = -1, by1 = -1;
    for (int y = 0; y < h; y++) for (int x = 0; x < w; x++) if (lines[y * w + x] > 20) { bx0 = Math.Min(bx0, x); bx1 = Math.Max(bx1, x); by0 = Math.Min(by0, y); by1 = Math.Max(by1, y); }
    int side = Math.Max(bx1 - bx0 + 1, by1 - by0 + 1); side += side / 50;
    int cx = (bx0 + bx1) / 2, cy = (by0 + by1) / 2;
    lines = CropSquare(lines, w, h, cx - side / 2, cy - side / 2, side);
    fill = CropSquare(fill, w, h, cx - side / 2, cy - side / 2, side);
    alpha = CropSquare(alpha, w, h, cx - side / 2, cy - side / 2, side);
    w = h = side;
    int inside = 0; foreach (var v in fill) if (v > 0) inside++;
    using (var l = Tinted(lines, w, h, Color.White)) SaveScaled(l, linesOut, 512, 0);
    using (var f = Tinted(fill, w, h, Color.White)) SaveScaled(f, fillOut, 512, 0);
    using (var a = Tinted(alpha, w, h, Color.White)) SaveScaled(a, lineArtOut, 512, 0);
    // The app icon: Suave colors (dark theme: lines #FF9068 over the head in #16131B), 2% margin.
    using (var icon = new Bitmap(w, h, PixelFormat.Format32bppArgb)) {
      using (var g = Graphics.FromImage(icon))
      using (var f = Tinted(fill, w, h, ColorTranslator.FromHtml("#16131B")))
      using (var l = Tinted(lines, w, h, ColorTranslator.FromHtml("#FF9068"))) { g.DrawImage(f, 0, 0, w, h); g.DrawImage(l, 0, 0, w, h); }
      SaveScaled(icon, appIconOut, 1024, 1024 * 2 / 100);
    }
    return string.Format("cropped to {0}x{0}; inside of the head: {1:P0} of the picture", side, inside / (double)(w * h));
  }
}
'@
New-Item -ItemType Directory -Force "$root\frontend\src\assets\brand" | Out-Null
[WolfIcon]::Run(
  "$root\frontend\src-tauri\app-icon-lineart.png",
  "$root\frontend\src\assets\brand\wolf-lines.png",
  "$root\frontend\src\assets\brand\wolf-fill.png",
  "$root\frontend\src\assets\brand\wolf-lineart.png",
  "$root\frontend\src-tauri\app-icon.png")
