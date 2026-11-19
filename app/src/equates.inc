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
libPage          = KD_SCRATCH
skClear          = 0Fh
onFlags          = 9
onInterrupt      = 4
graphFlags       = 3
graphDraw        = 0
