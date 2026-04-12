Add-Type -AssemblyName System.Drawing
$targetDir = 'c:\Users\jmema\OneDrive\Documentos\Proyectos\Zenit-Tauri\src-tauri\icons\nsis'
if (-not (Test-Path $targetDir)) { New-Item -Path $targetDir -ItemType Directory }

function Resize-Image($sourcePath, $targetPath, $width, $height) {
    $img = [System.Drawing.Image]::FromFile($sourcePath)
    $bmp = New-Object System.Drawing.Bitmap($width, $height)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.DrawImage($img, 0, 0, $width, $height)
    $g.Dispose()
    $img.Dispose()
    $bmp.Save($targetPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

Resize-Image 'C:\Users\jmema\.gemini\antigravity\brain\a2834319-28e9-4e37-b99d-9838aee0e1eb\sidebar_nano_1775981421548.png' "$targetDir\sidebar.png" 164 314
Resize-Image 'C:\Users\jmema\.gemini\antigravity\brain\a2834319-28e9-4e37-b99d-9838aee0e1eb\header_nano_1775981434826.png' "$targetDir\header.png" 57 57
Resize-Image 'C:\Users\jmema\.gemini\antigravity\brain\a2834319-28e9-4e37-b99d-9838aee0e1eb\banner_nano_1775981447492.png' "$targetDir\banner.png" 490 58
