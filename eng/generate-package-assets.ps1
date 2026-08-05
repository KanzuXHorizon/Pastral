[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

Add-Type -AssemblyName System.Drawing

function New-PastralAsset {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][int]$Width,
        [Parameter(Mandatory = $true)][int]$Height,
        [Parameter()][switch]$IncludeWordmark
    )

    $path = Join-Path $OutputDirectory $Name
    $bitmap = New-Object System.Drawing.Bitmap($Width, $Height, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

    $bounds = New-Object System.Drawing.Rectangle(0, 0, $Width, $Height)
    $violet = [System.Drawing.ColorTranslator]::FromHtml('#725CFF')
    $cyan = [System.Drawing.ColorTranslator]::FromHtml('#2ED3FF')
    $background = [System.Drawing.ColorTranslator]::FromHtml('#201A3A')
    $graphics.Clear($background)

    $gradient = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        $bounds,
        $violet,
        $cyan,
        [System.Drawing.Drawing2D.LinearGradientMode]::ForwardDiagonal
    )
    $graphics.FillRectangle($gradient, $bounds)

    $glowBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(42, 255, 255, 255))
    $glowSize = [Math]::Max(12, [int]([Math]::Min($Width, $Height) * 0.88))
    $graphics.FillEllipse(
        $glowBrush,
        [int](($Width - $glowSize) / 2),
        [int](($Height - $glowSize) / 2),
        $glowSize,
        $glowSize
    )

    $minimum = [Math]::Min($Width, $Height)
    $markHeight = [Math]::Max(10.0, $minimum * 0.48)
    $stroke = [Math]::Max(2.0, $minimum * 0.075)
    $markWidth = $markHeight * 0.72
    $centerX = if ($IncludeWordmark) { $Width * 0.31 } else { $Width * 0.5 }
    $centerY = $Height * 0.5
    $left = $centerX - ($markWidth * 0.45)
    $top = $centerY - ($markHeight * 0.5)

    $pen = New-Object System.Drawing.Pen([System.Drawing.Color]::White, [single]$stroke)
    $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    $graphics.DrawLine($pen, [single]$left, [single]($top + $markHeight), [single]$left, [single]$top)
    $graphics.DrawArc(
        $pen,
        [single]($left - ($stroke * 0.05)),
        [single]$top,
        [single]$markWidth,
        [single]($markHeight * 0.58),
        [single]270,
        [single]180
    )
    $graphics.DrawLine(
        $pen,
        [single]($left + ($markWidth * 0.50)),
        [single]($top + ($markHeight * 0.55)),
        [single]($left + ($markWidth * 0.78)),
        [single]($top + ($markHeight * 0.82))
    )

    if ($IncludeWordmark) {
        $fontSize = [Math]::Max(12.0, $Height * 0.17)
        $font = New-Object System.Drawing.Font('Segoe UI Semibold', [single]$fontSize, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
        $textBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
        $textX = [single]($Width * 0.43)
        $textY = [single](($Height - $fontSize) / 2 - ($fontSize * 0.08))
        $graphics.DrawString('Pastral', $font, $textBrush, $textX, $textY)
        $textBrush.Dispose()
        $font.Dispose()
    }

    $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)

    $pen.Dispose()
    $glowBrush.Dispose()
    $gradient.Dispose()
    $graphics.Dispose()
    $bitmap.Dispose()

    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Package asset was not generated: $path"
    }
}

New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
Get-ChildItem -LiteralPath $OutputDirectory -Filter '*.png' -File -ErrorAction SilentlyContinue |
    Remove-Item -Force

New-PastralAsset -Name 'StoreLogo.png' -Width 50 -Height 50
New-PastralAsset -Name 'Square44x44Logo.png' -Width 44 -Height 44
New-PastralAsset -Name 'Square150x150Logo.png' -Width 150 -Height 150
New-PastralAsset -Name 'Wide310x150Logo.png' -Width 310 -Height 150 -IncludeWordmark
New-PastralAsset -Name 'Square310x310Logo.png' -Width 310 -Height 310
New-PastralAsset -Name 'SplashScreen.png' -Width 620 -Height 300 -IncludeWordmark

Write-Host "Pastral package assets generated at $OutputDirectory"
