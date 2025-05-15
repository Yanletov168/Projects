param (
    [string]$BaseDirectory = "C:\Users",
    [string]$RelativeCachePath = "AppData\Local\Google\Chrome\User Data\Default\Cache",
    [string]$RelativeUserDataPath = "AppData\Local\Google\Chrome\User Data",
    [string]$TargetDirectory = "E:\ChromeProfiles"
)

function Clear-CacheForAllUsers {
    param (
        [string]$BaseDirectory,
        [string]$RelativeCachePath
    )

    if (Test-Path $BaseDirectory) {
        Get-ChildItem -Path $BaseDirectory | ForEach-Object {
            $UserCacheDirectory = Join-Path -Path $_.FullName -ChildPath $RelativeCachePath
            if (Test-Path $UserCacheDirectory) {
                Write-Host "Clearing cache for: $UserCacheDirectory"
                Get-ChildItem -Path $UserCacheDirectory -Force | ForEach-Object {
                    try {
                        if ($_.PSIsContainer) {
                            Remove-Item -Path $_.FullName -Recurse -Force
                        } else {
                            Remove-Item -Path $_.FullName -Force
                        }
                    } catch {
                        Write-Host "Failed to delete $($_.FullName). Reason: $($_.Exception.Message)"
                    }
                }
            } else {
                Write-Host "Cache directory does not exist for: $($_.Name)"
            }
        }
    } else {
        Write-Host "The base directory $BaseDirectory does not exist."
    }
}

function Copy-UserDataToProfiles {
    param (
        [string]$BaseDirectory,
        [string]$RelativeUserDataPath,
        [string]$TargetDirectory
    )

    if (Test-Path $BaseDirectory) {
        Get-ChildItem -Path $BaseDirectory | ForEach-Object {
            $UserDataDirectory = Join-Path -Path $_.FullName -ChildPath $RelativeUserDataPath
            $TargetUserDirectory = Join-Path -Path $TargetDirectory -ChildPath $_.Name
            if (Test-Path $UserDataDirectory) {
                try {
                    if (-not (Test-Path $TargetUserDirectory)) {
                        New-Item -ItemType Directory -Path $TargetUserDirectory | Out-Null
                        Write-Host "Created directory for user: $TargetUserDirectory"
                    }
                    Copy-Item -Path $UserDataDirectory -Destination $TargetUserDirectory -Recurse -Force
                    Write-Host "Copied User Data for $($_.Name) to $TargetUserDirectory"
                } catch {
                    Write-Host "Failed to copy User Data for $($_.Name). Reason: $($_.Exception.Message)"
                }
            } else {
                Write-Host "User Data directory does not exist for: $($_.Name)"
            }
        }
    } else {
        Write-Host "The base directory $BaseDirectory does not exist."
    }
}

function Replace-UserDataWithLink {
    param (
        [string]$BaseDirectory,
        [string]$RelativeUserDataPath,
        [string]$TargetDirectory
    )

    if (Test-Path $BaseDirectory) {
        Get-ChildItem -Path $BaseDirectory | ForEach-Object {
            $UserDataDirectory = Join-Path -Path $_.FullName -ChildPath $RelativeUserDataPath
            $TargetUserDirectory = Join-Path -Path $TargetDirectory -ChildPath $_.Name
            if (Test-Path $UserDataDirectory) {
                try {
                    Remove-Item -Path $UserDataDirectory -Recurse -Force
                    Write-Host "Deleted original User Data directory: $UserDataDirectory"

                    cmd.exe /c "mklink /J `"$UserDataDirectory`" `"$TargetUserDirectory`""
                    Write-Host "Created junction link from $UserDataDirectory to $TargetUserDirectory"
                } catch {
                    Write-Host "Failed to replace User Data with link for $($_.Name). Reason: $($_.Exception.Message)"
                }
            } else {
                Write-Host "User Data directory does not exist for: $($_.Name)"
            }
        }
    } else {
        Write-Host "The base directory $BaseDirectory does not exist."
    }
}

# Очистка кеша
Clear-CacheForAllUsers -BaseDirectory $BaseDirectory -RelativeCachePath $RelativeCachePath

# Копирование папок User Data
Copy-UserDataToProfiles -BaseDirectory $BaseDirectory -RelativeUserDataPath $RelativeUserDataPath -TargetDirectory $TargetDirectory

# Замена папок User Data на жесткие ссылки
Replace-UserDataWithLink -BaseDirectory $BaseDirectory -RelativeUserDataPath $RelativeUserDataPath -TargetDirectory $TargetDirectory