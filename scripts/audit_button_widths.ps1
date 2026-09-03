# 检查所有 .slint 文件中带有固定 width 的按钮或卡片中的 Text 元素
$files = Get-ChildItem -Path "crates/smagical-ui/ui" -Filter "*.slint" -Recurse

Write-Host "Auditing fixed-width containers containing Text..."

foreach ($file in $files) {
    $lines = Get-Content $file.FullName -Encoding UTF8
    for ($i = 0; $i -lt $lines.Length; $i++) {
        $l = $lines[$i]
        # 查找形如 width: 70px ~ 160px 的声明
        if ($l -match '^\s*width:\s*(\d+)px;') {
            $w = [int]$Matches[1]
            if ($w -ge 40 -and $w -le 160) {
                # 检查接下来的 25 行内是否有 Text 包含 @tr 或中文
                $foundText = $null
                $maxLookahead = [Math]::Min($lines.Length - 1, $i + 25)
                for ($j = $i + 1; $j -le $maxLookahead; $j++) {
                    if ($lines[$j] -match 'text:\s*(@tr\(.*?\)|".*?")') {
                        $foundText = $Matches[1]
                        $textLine = $j + 1
                        break
                    }
                    # 如果遇到同级闭合括号或新的组件声明，停止扫描
                    if ($lines[$j] -match '^\s*(Rectangle|VerticalLayout|HorizontalLayout)\s*\{') {
                        # 子布局继续，其他停止
                    }
                }
                if ($foundText -and ($foundText -match '@tr' -or $foundText -match '[\p{IsCJKUnifiedIdeographs}]')) {
                    $relPath = $file.FullName.Substring($file.FullName.IndexOf("crates\smagical-ui"))
                    Write-Host "[$relPath : L$($i + 1)] width: ${w}px -> Text at L$textLine : $foundText"
                }
            }
        }
    }
}
