; The shell's constant strings, as glyph codes ended by 0, and the soft key table. They are linked
; on the draw page, the page the canvas reads a string from; the entry page passes their addresses
; to the canvas's far entries. The About screen's version is VERSION_MAJOR, VERSION_MINOR and
; VERSION_PATCH, which the Makefile takes from the workspace version.
str_title:
	.db UI_FONT_CH_6D, UI_FONT_CH_65, UI_FONT_CH_74, UI_FONT_CH_61, UI_FONT_CH_63, UI_FONT_CH_61, UI_FONT_CH_6C, UI_FONT_CH_63, 0
str_none:
	.db 0
str_hint:
	.db UI_FONT_CH_43, UI_FONT_CH_4C, UI_FONT_CH_45, UI_FONT_CH_41, UI_FONT_CH_52, UI_FONT_CH_20, UI_FONT_CH_74, UI_FONT_CH_6F, UI_FONT_CH_20, UI_FONT_CH_71, UI_FONT_CH_75, UI_FONT_CH_69, UI_FONT_CH_74, 0
str_theorems:
	.db UI_FONT_CH_54, UI_FONT_CH_68, UI_FONT_CH_65, UI_FONT_CH_6F, UI_FONT_CH_72, UI_FONT_CH_65, UI_FONT_CH_6D, UI_FONT_CH_73, 0
str_verify:
	.db UI_FONT_CH_56, UI_FONT_CH_65, UI_FONT_CH_72, UI_FONT_CH_69, UI_FONT_CH_66, UI_FONT_CH_79, 0
str_error:
	.db UI_FONT_CH_45, UI_FONT_CH_72, UI_FONT_CH_72, UI_FONT_CH_6F, UI_FONT_CH_72, 0
str_unavailable:
	.db UI_FONT_CH_4E, UI_FONT_CH_6F, UI_FONT_CH_74, UI_FONT_CH_20, UI_FONT_CH_61, UI_FONT_CH_76, UI_FONT_CH_61, UI_FONT_CH_69, UI_FONT_CH_6C, UI_FONT_CH_61, UI_FONT_CH_62, UI_FONT_CH_6C, UI_FONT_CH_65, UI_FONT_CH_20, UI_FONT_CH_79, UI_FONT_CH_65, UI_FONT_CH_74, 0
str_about:
	.db UI_FONT_CH_41, UI_FONT_CH_62, UI_FONT_CH_6F, UI_FONT_CH_75, UI_FONT_CH_74, 0
str_about_a:
	.db UI_FONT_CH_4D, UI_FONT_CH_65, UI_FONT_CH_74, UI_FONT_CH_61, UI_FONT_CH_6D, UI_FONT_CH_61, UI_FONT_CH_74, UI_FONT_CH_68, UI_FONT_CH_20, UI_FONT_CH_76, UI_FONT_CH_65, UI_FONT_CH_72, UI_FONT_CH_69, UI_FONT_CH_66, UI_FONT_CH_69, UI_FONT_CH_65, UI_FONT_CH_72, 0
str_about_b:
	.db UI_FONT_CH_61, UI_FONT_CH_6E, UI_FONT_CH_64, UI_FONT_CH_20, UI_FONT_CH_70, UI_FONT_CH_72, UI_FONT_CH_6F, UI_FONT_CH_6F, UI_FONT_CH_66, UI_FONT_CH_20, UI_FONT_CH_65, UI_FONT_CH_64, UI_FONT_CH_69, UI_FONT_CH_74, UI_FONT_CH_6F, UI_FONT_CH_72, 0
#ifdef LIBRARY
str_about_libs:
	.db UI_FONT_CH_31, UI_FONT_CH_20, UI_FONT_CH_6C, UI_FONT_CH_69, UI_FONT_CH_62, UI_FONT_CH_72, UI_FONT_CH_61, UI_FONT_CH_72, UI_FONT_CH_79, 0
#else
str_about_libs:
	.db UI_FONT_CH_30, UI_FONT_CH_20, UI_FONT_CH_6C, UI_FONT_CH_69, UI_FONT_CH_62, UI_FONT_CH_72, UI_FONT_CH_61, UI_FONT_CH_72, UI_FONT_CH_69, UI_FONT_CH_65, UI_FONT_CH_73, 0
#endif
str_back:
	.db UI_KEY_FONT_CH_42, UI_KEY_FONT_CH_61, UI_KEY_FONT_CH_63, UI_KEY_FONT_CH_6B, 0
str_keys_back:
	.dw str_back, str_none, str_none, str_none, str_none

#ifndef VERSION_MAJOR
 .error "VERSION_MAJOR, VERSION_MINOR and VERSION_PATCH name the version, as the Makefile passes them"
#endif
#if (UI_FONT_CH_39 - UI_FONT_CH_30) != 9
 .error "the digits 0 to 9 are not consecutive glyph codes"
#endif
#macro VERSION_PART(n)
#if (n) >= 100
	.db UI_FONT_CH_30 + ((n) / 100)
#endif
#if (n) >= 10
	.db UI_FONT_CH_30 + (((n) / 10) % 10)
#endif
	.db UI_FONT_CH_30 + ((n) % 10)
#endmacro
str_about_version:
	.db UI_FONT_CH_6D, UI_FONT_CH_65, UI_FONT_CH_74, UI_FONT_CH_61, UI_FONT_CH_63, UI_FONT_CH_61, UI_FONT_CH_6C, UI_FONT_CH_63, UI_FONT_CH_20
	VERSION_PART(VERSION_MAJOR)
	.db UI_FONT_CH_2E
	VERSION_PART(VERSION_MINOR)
	.db UI_FONT_CH_2E
	VERSION_PART(VERSION_PATCH)
	.db 0
