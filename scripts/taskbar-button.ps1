# Saves a picture of just the Jay-S button on the taskbar (cropped by its UI Automation rectangle, so
# nothing else on the screen is captured):  powershell -File scripts/taskbar-button.ps1 <out.png>
param([string]$Out = "$env:TEMP\jays-taskbar.png")
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing
$root = [System.Windows.Automation.AutomationElement]::RootElement
$tray = $root.FindFirst([System.Windows.Automation.TreeScope]::Children, (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ClassNameProperty, 'Shell_TrayWnd')))
$buttons = $tray.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { $_.Current.Name -like 'Jay-S*' -and $_.Current.ControlType.ProgrammaticName -like '*Button*' }
if (-not $buttons) { "no Jay-S taskbar button"; exit 1 }
$r = @($buttons)[0].Current.BoundingRectangle
$bmp = New-Object System.Drawing.Bitmap ([int]$r.Width), ([int]$r.Height)
[System.Drawing.Graphics]::FromImage($bmp).CopyFromScreen([int]$r.X, [int]$r.Y, 0, 0, $bmp.Size)
$bmp.Save($Out)
"{0} ({1}x{2}) name: {3}" -f $Out, [int]$r.Width, [int]$r.Height, @($buttons)[0].Current.Name
