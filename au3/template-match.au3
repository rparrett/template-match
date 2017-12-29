Global $templateMatchDll = DllOpen("template_match.dll");

Func TemplateMatch($filename)
    Local $struct_TemplateMatchResult = "struct;uint x;uint y;double rms;uint err;endstruct;"
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

    $error = DllCall($templateMatchDll, "uint", "template_match", "str", $filename, "struct*", $templateMatchResult)

	$result[0] = DllStructGetData($templateMatchResult, "x")
	$result[1] = DllStructGetData($templateMatchResult, "y")
	$result[2] = DllStructGetData($templateMatchResult, "rms")
	$result[3] = $error;

	Return $result;
EndFunc