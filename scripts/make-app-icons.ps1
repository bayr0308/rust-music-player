# Convert the approved artwork to runtime PNG and multi-resolution Windows ICO.
# Requires only System.Drawing, included with Windows PowerShell / .NET on Windows.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$projectRoot = Split-Path -Parent $PSScriptRoot
$assetsPath = Join-Path $projectRoot 'assets'
$sourcePath = Join-Path $assetsPath 'mellow-icon.png'
$pngPath = Join-Path $assetsPath 'mellow-icon-256.png'
$icoPath = Join-Path $assetsPath 'mellow-icon.ico'
$sizes = @(16, 20, 24, 32, 40, 48, 64, 128, 256)
$frames = [System.Collections.Generic.List[byte[]]]::new()
$source = [System.Drawing.Image]::FromFile($sourcePath)
try {
    if ($source.Width -ne $source.Height) { throw 'The source app icon must be square.' }
    foreach ($size in $sizes) {
        $bitmap = [System.Drawing.Bitmap]::new($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $stream = [System.IO.MemoryStream]::new()
        $attributes = [System.Drawing.Imaging.ImageAttributes]::new()
        try {
            $graphics.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
            $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
            $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
            # Extend the edge pixels so resizing does not introduce a translucent border.
            $attributes.SetWrapMode([System.Drawing.Drawing2D.WrapMode]::TileFlipXY)
            $graphics.DrawImage($source, [System.Drawing.Rectangle]::new(0, 0, $size, $size), 0, 0, $source.Width, $source.Height, [System.Drawing.GraphicsUnit]::Pixel, $attributes)
            $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
            $frames.Add($stream.ToArray())
            if ($size -eq 256) { [System.IO.File]::WriteAllBytes($pngPath, $stream.ToArray()) }
        } finally {
            $attributes.Dispose()
            $stream.Dispose()
            $graphics.Dispose()
            $bitmap.Dispose()
        }
    }
} finally {
    $source.Dispose()
}

$file = [System.IO.File]::Create($icoPath)
$writer = [System.IO.BinaryWriter]::new($file)
try {
    $writer.Write([uint16]0) # Reserved
    $writer.Write([uint16]1) # Icon
    $writer.Write([uint16]$sizes.Count)
    $offset = 6 + 16 * $sizes.Count
    for ($i = 0; $i -lt $sizes.Count; $i++) {
        $dimension = if ($sizes[$i] -eq 256) { 0 } else { $sizes[$i] }
        $writer.Write([byte]$dimension)
        $writer.Write([byte]$dimension)
        $writer.Write([byte]0) # RGBA, no palette
        $writer.Write([byte]0)
        $writer.Write([uint16]1) # Color planes
        $writer.Write([uint16]32) # Bits per pixel
        $writer.Write([uint32]$frames[$i].Length)
        $writer.Write([uint32]$offset)
        $offset += $frames[$i].Length
    }
    foreach ($frame in $frames) { $writer.Write([byte[]]$frame) }
} finally {
    $writer.Dispose()
    $file.Dispose()
}
Write-Output "Created $pngPath and $icoPath (sizes: $($sizes -join ', '))."
