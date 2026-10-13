" Filetype settings for Cadetta.
if exists('b:did_ftplugin')
  finish
endif
let b:did_ftplugin = 1

setlocal commentstring=//\ %s
setlocal comments=:///,://
setlocal formatoptions-=t formatoptions+=croql
setlocal iskeyword+=-
setlocal expandtab shiftwidth=4 softtabstop=4
compiler cadetta
let b:undo_ftplugin = 'setlocal commentstring< comments< formatoptions< iskeyword< expandtab< shiftwidth< softtabstop<'
