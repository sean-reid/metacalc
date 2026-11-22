; The shell's constant strings, as glyph codes ended by 0, in one block for each screen. The canvas
; reads a string with its own page mapped, so a screen's draw copies its block to APP_STAGE with
; app_stage_block (text.inc) and hands the canvas the copy's addresses, the stg_ labels: a string's
; address in the block less the block's start, plus APP_STAGE_DATA. A block of more than
; APP_STAGE_DATA_SIZE bytes stops the assembly. A screen's strings that are built at run time are in
; APP_BUF and are handed over where they are. The About screen's version is VERSION_MAJOR,
; VERSION_MINOR and VERSION_PATCH, which the Makefile takes from the workspace version.
#ifndef VERSION_MAJOR
 .error "VERSION_MAJOR, VERSION_MINOR and VERSION_PATCH name the version, as the Makefile passes them"
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

start_block:
ss_title:
	.db UI_FONT_CH_6D, UI_FONT_CH_65, UI_FONT_CH_74, UI_FONT_CH_61, UI_FONT_CH_63, UI_FONT_CH_61, UI_FONT_CH_6C, UI_FONT_CH_63, 0
ss_none:
	.db 0
ss_hint:
	.db UI_FONT_CH_43, UI_FONT_CH_4C, UI_FONT_CH_45, UI_FONT_CH_41, UI_FONT_CH_52, UI_FONT_CH_20, UI_FONT_CH_74, UI_FONT_CH_6F, UI_FONT_CH_20, UI_FONT_CH_71, UI_FONT_CH_75, UI_FONT_CH_69, UI_FONT_CH_74, 0
start_block_end:
STG_START_TITLE	.equ APP_STAGE_DATA + (ss_title - start_block)
STG_START_NONE	.equ APP_STAGE_DATA + (ss_none - start_block)
STG_START_HINT	.equ APP_STAGE_DATA + (ss_hint - start_block)
START_BLOCK_SIZE	.equ start_block_end - start_block
#if START_BLOCK_SIZE > APP_STAGE_DATA_SIZE
 .error "the start screen's strings are longer than APP_STAGE_DATA_SIZE"
#endif

notice_block:
ns_theorems:
	.db UI_FONT_CH_54, UI_FONT_CH_68, UI_FONT_CH_65, UI_FONT_CH_6F, UI_FONT_CH_72, UI_FONT_CH_65, UI_FONT_CH_6D, UI_FONT_CH_73, 0
ns_verify:
	.db UI_FONT_CH_56, UI_FONT_CH_65, UI_FONT_CH_72, UI_FONT_CH_69, UI_FONT_CH_66, UI_FONT_CH_79, 0
ns_error:
	.db UI_FONT_CH_45, UI_FONT_CH_72, UI_FONT_CH_72, UI_FONT_CH_6F, UI_FONT_CH_72, 0
ns_unavailable:
	.db UI_FONT_CH_4E, UI_FONT_CH_6F, UI_FONT_CH_74, UI_FONT_CH_20, UI_FONT_CH_61, UI_FONT_CH_76, UI_FONT_CH_61, UI_FONT_CH_69, UI_FONT_CH_6C, UI_FONT_CH_61, UI_FONT_CH_62, UI_FONT_CH_6C, UI_FONT_CH_65, UI_FONT_CH_20, UI_FONT_CH_79, UI_FONT_CH_65, UI_FONT_CH_74, 0
ns_none:
	.db 0
ns_back:
	.db UI_KEY_FONT_CH_42, UI_KEY_FONT_CH_61, UI_KEY_FONT_CH_63, UI_KEY_FONT_CH_6B, 0
ns_keys:
	.dw STG_NOTICE_BACK, STG_NOTICE_NONE, STG_NOTICE_NONE, STG_NOTICE_NONE, STG_NOTICE_NONE
notice_block_end:
STG_NOTICE_THEOREMS	.equ APP_STAGE_DATA + (ns_theorems - notice_block)
STG_NOTICE_VERIFY	.equ APP_STAGE_DATA + (ns_verify - notice_block)
STG_NOTICE_ERROR	.equ APP_STAGE_DATA + (ns_error - notice_block)
STG_NOTICE_UNAVAILABLE	.equ APP_STAGE_DATA + (ns_unavailable - notice_block)
STG_NOTICE_NONE	.equ APP_STAGE_DATA + (ns_none - notice_block)
STG_NOTICE_BACK	.equ APP_STAGE_DATA + (ns_back - notice_block)
STG_NOTICE_KEYS	.equ APP_STAGE_DATA + (ns_keys - notice_block)
NOTICE_BLOCK_SIZE	.equ notice_block_end - notice_block
#if NOTICE_BLOCK_SIZE > APP_STAGE_DATA_SIZE
 .error "the notices' strings are longer than APP_STAGE_DATA_SIZE"
#endif

about_block:
as_title:
	.db UI_FONT_CH_41, UI_FONT_CH_62, UI_FONT_CH_6F, UI_FONT_CH_75, UI_FONT_CH_74, 0
as_none:
	.db 0
as_version:
	.db UI_FONT_CH_6D, UI_FONT_CH_65, UI_FONT_CH_74, UI_FONT_CH_61, UI_FONT_CH_63, UI_FONT_CH_61, UI_FONT_CH_6C, UI_FONT_CH_63, UI_FONT_CH_20
	VERSION_PART(VERSION_MAJOR)
	.db UI_FONT_CH_2E
	VERSION_PART(VERSION_MINOR)
	.db UI_FONT_CH_2E
	VERSION_PART(VERSION_PATCH)
	.db 0
as_a:
	.db UI_FONT_CH_4D, UI_FONT_CH_65, UI_FONT_CH_74, UI_FONT_CH_61, UI_FONT_CH_6D, UI_FONT_CH_61, UI_FONT_CH_74, UI_FONT_CH_68, UI_FONT_CH_20, UI_FONT_CH_76, UI_FONT_CH_65, UI_FONT_CH_72, UI_FONT_CH_69, UI_FONT_CH_66, UI_FONT_CH_69, UI_FONT_CH_65, UI_FONT_CH_72, 0
as_b:
	.db UI_FONT_CH_61, UI_FONT_CH_6E, UI_FONT_CH_64, UI_FONT_CH_20, UI_FONT_CH_70, UI_FONT_CH_72, UI_FONT_CH_6F, UI_FONT_CH_6F, UI_FONT_CH_66, UI_FONT_CH_20, UI_FONT_CH_65, UI_FONT_CH_64, UI_FONT_CH_69, UI_FONT_CH_74, UI_FONT_CH_6F, UI_FONT_CH_72, 0
#ifdef LIBRARY
as_libs:
	.db UI_FONT_CH_31, UI_FONT_CH_20, UI_FONT_CH_6C, UI_FONT_CH_69, UI_FONT_CH_62, UI_FONT_CH_72, UI_FONT_CH_61, UI_FONT_CH_72, UI_FONT_CH_79, 0
#else
as_libs:
	.db UI_FONT_CH_30, UI_FONT_CH_20, UI_FONT_CH_6C, UI_FONT_CH_69, UI_FONT_CH_62, UI_FONT_CH_72, UI_FONT_CH_61, UI_FONT_CH_72, UI_FONT_CH_69, UI_FONT_CH_65, UI_FONT_CH_73, 0
#endif
about_block_end:
STG_ABOUT_TITLE	.equ APP_STAGE_DATA + (as_title - about_block)
STG_ABOUT_NONE	.equ APP_STAGE_DATA + (as_none - about_block)
STG_ABOUT_VERSION	.equ APP_STAGE_DATA + (as_version - about_block)
STG_ABOUT_A	.equ APP_STAGE_DATA + (as_a - about_block)
STG_ABOUT_B	.equ APP_STAGE_DATA + (as_b - about_block)
STG_ABOUT_LIBS	.equ APP_STAGE_DATA + (as_libs - about_block)
ABOUT_BLOCK_SIZE	.equ about_block_end - about_block
#if ABOUT_BLOCK_SIZE > APP_STAGE_DATA_SIZE
 .error "the About screen's strings are longer than APP_STAGE_DATA_SIZE"
#endif
