# Logs in to SimplySign Desktop on the Windows release runner, so signtool can use the
# Certum cloud certificate. The 6-digit code comes from the TOTP seed in the SimplySign QR code.
#
# Env: CERTUM_USER       SimplySign login (email)
#      CERTUM_OTP_URI    otpauth://totp/...?secret=... from the QR code (or only the base32 secret)
#      CERTUM_CERT_SHA1  thumbprint of the code signing certificate
$ErrorActionPreference = 'Stop'

function Get-OtpParameters([string]$uri) {
    $result = @{ Secret = $uri; Digits = 6; Period = 30 }
    if ($uri -match '^otpauth://') {
        $query = $uri.Split('?', 2)[1]
        foreach ($pair in $query.Split('&')) {
            $key, $value = $pair.Split('=', 2)
            $value = [Uri]::UnescapeDataString($value)
            switch ($key.ToLower()) {
                'secret' { $result.Secret = $value }
                'digits' { $result.Digits = [int]$value }
                'period' { $result.Period = [int]$value }
                'algorithm' { if ($value.ToUpper() -ne 'SHA1') { throw "Only SHA1 codes are supported, not $value" } }
            }
        }
    }
    $result
}

function ConvertFrom-Base32([string]$text) {
    $alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'
    $text = $text.ToUpper().TrimEnd('=').Replace(' ', '')
    $bytes = New-Object System.Collections.Generic.List[byte]
    $buffer = 0
    $bits = 0
    foreach ($char in $text.ToCharArray()) {
        $index = $alphabet.IndexOf($char)
        if ($index -lt 0) { throw "The TOTP secret is not base32" }
        $buffer = ($buffer -shl 5) -bor $index
        $bits += 5
        if ($bits -ge 8) {
            $bits -= 8
            $bytes.Add([byte](($buffer -shr $bits) -band 0xFF))
        }
    }
    $bytes.ToArray()
}

function Get-TotpCode($otp, [long]$unixTime) {
    $counter = [BitConverter]::GetBytes([long][Math]::Floor($unixTime / $otp.Period))
    if ([BitConverter]::IsLittleEndian) { [Array]::Reverse($counter) }
    $hmac = New-Object System.Security.Cryptography.HMACSHA1 -ArgumentList (, (ConvertFrom-Base32 $otp.Secret))
    $hash = $hmac.ComputeHash($counter)
    $offset = $hash[$hash.Length - 1] -band 0x0F
    $number = (([int]$hash[$offset] -band 0x7F) -shl 24) -bor ([int]$hash[$offset + 1] -shl 16) -bor ([int]$hash[$offset + 2] -shl 8) -bor [int]$hash[$offset + 3]
    ($number % [Math]::Pow(10, $otp.Digits)).ToString().PadLeft($otp.Digits, '0')
}

foreach ($name in 'CERTUM_USER', 'CERTUM_OTP_URI', 'CERTUM_CERT_SHA1') {
    if (-not [Environment]::GetEnvironmentVariable($name)) { throw "$name is not set" }
}
$thumbprint = $env:CERTUM_CERT_SHA1.Replace(' ', '').ToUpper()
$otp = Get-OtpParameters $env:CERTUM_OTP_URI

$exe = Get-ChildItem "$env:ProgramFiles\Certum", "${env:ProgramFiles(x86)}\Certum" -Recurse -Filter SimplySignDesktop.exe -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $exe) { throw 'SimplySignDesktop.exe not found' }

# Windows copies smart card certificates into the user's store only while this service runs.
Start-Service CertPropSvc -ErrorAction SilentlyContinue

Start-Process $exe.FullName
$shell = New-Object -ComObject WScript.Shell
$deadline = (Get-Date).AddSeconds(60)
while (-not $shell.AppActivate('SimplySign Desktop')) {
    if ((Get-Date) -gt $deadline) { throw 'The SimplySign Desktop login window did not open' }
    Start-Sleep -Seconds 1
}
Start-Sleep -Seconds 2

# Wait for a fresh code if this one is about to run out.
$now = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
if ($otp.Period - ($now % $otp.Period) -lt 5) {
    Start-Sleep -Seconds ($otp.Period - ($now % $otp.Period) + 1)
    $now = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
}
$code = Get-TotpCode $otp $now

$shell.AppActivate('SimplySign Desktop') | Out-Null
$shell.SendKeys(($env:CERTUM_USER -replace '([+^%~(){}\[\]])', '{$1}'))
$shell.SendKeys('{TAB}')
$shell.SendKeys($code)
$shell.SendKeys('{ENTER}')

$deadline = (Get-Date).AddSeconds(60)
while (-not (Test-Path "Cert:\CurrentUser\My\$thumbprint")) {
    if ((Get-Date) -gt $deadline) { throw 'The certificate did not show in the user store after the SimplySign login' }
    Start-Sleep -Seconds 2
}
Write-Host 'SimplySign Desktop is logged in and the certificate is ready'
