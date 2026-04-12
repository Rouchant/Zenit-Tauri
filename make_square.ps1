Add-Type -AssemblyName System.Drawing
$img = [System.Drawing.Image]::FromFile('c:\Users\jmema\OneDrive\Documentos\Proyectos\Zenit-Tauri\public\assets\logo.png')
$minDim = if ($img.Width -lt $img.Height) { $img.Width } else { $img.Height }
$squareImg = New-Object System.Drawing.Bitmap($minDim, $minDim)
$g = [System.Drawing.Graphics]::FromImage($squareImg)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$srcRect = New-Object System.Drawing.Rectangle(([int](($img.Width - $minDim) / 2)), ([int](($img.Height - $minDim) / 2)), $minDim, $minDim)
$destRect = New-Object System.Drawing.Rectangle(0, 0, $minDim, $minDim)
$g.DrawImage($img, $destRect, $srcRect, [System.Drawing.GraphicsUnit]::Pixel)
$g.Dispose()
$img.Dispose()
$squareImg.Save('c:\Users\jmema\OneDrive\Documentos\Proyectos\Zenit-Tauri\public\assets\logo_square.png', [System.Drawing.Imaging.ImageFormat]::Png)
$squareImg.Dispose()
