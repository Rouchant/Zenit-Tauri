Add-Type -AssemblyName System.Drawing
$targetDir = 'c:\Users\jmema\OneDrive\Documentos\Proyectos\Zenit-Tauri\src-tauri\icons\nsis'

function Convert-ToBmp($sourcePath, $targetPath) {
    $img = [System.Drawing.Image]::FromFile($sourcePath)
    $img.Save($targetPath, [System.Drawing.Imaging.ImageFormat]::Bmp)
    $img.Dispose()
}

if (Test-Path "$targetDir\sidebar.png") {
    Convert-ToBmp "$targetDir\sidebar.png" "$targetDir\sidebar.bmp"
}
if (Test-Path "$targetDir\header.png") {
    Convert-ToBmp "$targetDir\header.png" "$targetDir\header.bmp"
}
