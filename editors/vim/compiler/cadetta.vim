" Compiler: cadetta check, short output (file:line:col: severity[CODE]: message).
if exists('current_compiler')
  finish
endif
let current_compiler = 'cadetta'

CompilerSet makeprg=cadetta\ check\ --format\ short
CompilerSet errorformat=%f:%l:%c:\ %t%*[a-z][%*[^]]]:\ %m
