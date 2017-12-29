Global $templateMatchDll = DllOpen("template_match.dll");
Global $struct_TemplateMatchResult = "struct;uint x;uint y;double rms;uint err;endstruct;"
Global $struct_Rect = "struct;uint x;uint y;uint w; uint h;endstruct;"

Func TemplateMatch($filename)
    Local $templateMatchResult = DllStructCreate($struct_TemplateMatchResult)

	Local $result[4]
	$result[0] = 0
	$result[1] = 0
	$result[2] = 0.0
	$result[3] = 0

    If @error Then
		$result[3] = 10
		Return $result
    EndIf

    $error = DllCall($templateMatchDll, "uint", "template_match", "str", $filename, "ptr", 0, "struct*", $templateMatchResult)

	$result[0] = DllStructGetData($templateMatchResult, "x")
	$result[1] = DllStructGetData($templateMatchResult, "y")
	$result[2] = DllStructGetData($templateMatchResult, "rms")
	$result[3] = $error

	Return $result;
EndFunc

Func TemplateMatchRect($filename, $x, $y, $w, $h)
    Local $rect = DllStructCreate($struct_Rect)
    DllStructSetData($rect, "x", $x)
    DllStructSetData($rect, "y", $y)
    DllStructSetData($rect, "w", $w)
    DllStructSetData($rect, "h", $h)

	Local $templateMatchResult = DllStructCreate($struct_TemplateMatchResult)

    Local $result[4]
    $result[0] = 0
    $result[1] = 0
    $result[2] = 0.0
    $result[3] = 0

    If @error Then
        $result[3] = 10
        Return $result
    EndIf

    $error = DllCall($templateMatchDll, "uint", "template_match", "str", $filename, "struct*", $rect, "struct*", $templateMatchResult)

    $result[0] = DllStructGetData($templateMatchResult, "x")
    $result[1] = DllStructGetData($templateMatchResult, "y")
    $result[2] = DllStructGetData($templateMatchResult, "rms")
    $result[3] = $error

    Return $result;
EndFunc
