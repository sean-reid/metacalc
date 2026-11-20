#include "ram_map.inc"

#define bcall(addr) rst 28h \ .dw addr
#define bjump(addr) call 50h \ .dw addr

_GetCSC          = 4018h
_ChkFindSym      = 42F1h
_EnoughMem       = 42FDh
_CreateAppVar    = 4E6Ah
_CreateRList     = 4315h
_DelVarArc       = 4FC6h
_Arc_Unarc       = 4FD8h
_LoadDEIndPaged  = 501Ah
_JForceCmdNoChar = 4027h
_PutC            = 4504h
_ClrLCDFull      = 4540h
_GrBufCpy        = 486Ah
_GetKey          = 4972h
curRow           = 844Bh
curCol           = 844Ch
_NewLine         = 452Eh
OP1              = 8478h
OP2              = 8483h
OP3              = 848Eh
appVarObj        = 15h
plotSScreen      = 9340h
appBackUpScreen  = 9872h
#if (plotSScreen < UI_FRAME) || (plotSScreen > UI_FRAME)
 .error "UI_FRAME in ram_map.inc must be plotSScreen"
#endif
#if (appBackUpScreen < KD_SCRATCH) || (appBackUpScreen > KD_SCRATCH)
 .error "KD_SCRATCH in ram_map.inc must be appBackUpScreen"
#endif
smallEditRAM     = 90D3h
XFact            = 913Fh
cmdShadow        = 966Eh
cmdShadCur       = 96EEh
#if (smallEditRAM < SCR_VARS) || (smallEditRAM > SCR_VARS)
 .error "SCR_VARS in ram_map.inc must be smallEditRAM"
#endif
#if (XFact < (SCR_VARS + SCR_VARS_SIZE)) || (XFact > (SCR_VARS + SCR_VARS_SIZE))
 .error "SCR_VARS in ram_map.inc must end where XFact starts"
#endif
#if (cmdShadow < SCR_STACK) || (cmdShadow > SCR_STACK)
 .error "SCR_STACK in ram_map.inc must be cmdShadow"
#endif
#if (cmdShadCur < (SCR_STACK + SCR_STACK_SIZE)) || (cmdShadCur > (SCR_STACK + SCR_STACK_SIZE))
 .error "SCR_STACK in ram_map.inc must end where cmdShadCur starts"
#endif
libPage          = KD_SCRATCH
skClear          = 0Fh
onFlags          = 9
onInterrupt      = 4
graphFlags       = 3
graphDraw        = 0
kClear           = 09h
scriptFlags      = 33h
_GetBaseVer      = 4C6Fh
appData          = 8000h
APP_DATA_SIZE    = 256
textShadow       = 8508h
TEXT_SHADOW_SIZE = 128
saveSScreen      = 86ECh
SAVE_SCREEN_SIZE = 768
statVars         = 8A3Ah
STAT_VARS_SIZE   = 531
prevData         = 9198h
PREV_DATA_SIZE   = 283
