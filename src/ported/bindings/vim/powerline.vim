" powerliners — vim statusline / tabline driver.
"
" Shipped inside the `powerliners` binary via `include_str!` and
" extracted to `$XDG_CACHE_HOME/powerliners/vim/powerline.vim` on
" first call. Source it from your `.vimrc` with:
"
"   set runtimepath+=~/.cache/powerliners/vim
"   source ~/.cache/powerliners/vim/powerline.vim
"
" Or, faster — install via the launcher in `.vimrc`:
"
"   if executable('powerline-config')
"     execute 'source' trim(system('powerline-config vim source-path'))
"   endif
"
" Wire format (see `src/bin/shared/render_runtime.rs::render_once`
" vim branch): the response from the daemon / `powerline-render` is
" a UTF-8 byte stream of the form
"
"   hi GroupA ctermfg=... ctermbg=...
"   hi GroupB ctermfg=... ctermbg=...
"                                       <-- empty separator line
"   %#GroupA# segment 1 %#GroupB# segment 2 ...
"
" Each `hi` line is executed via `:execute`, the empty line is the
" delimiter, and the final line is assigned to `&statusline`. The
" daemon caches its `VimRenderer` state across requests so once a hl
" group has been declared the response stops repeating it.
"
" Uses vim8-compatible syntax (functions, `let`, `s:` script-locals)
" so the plugin loads identically on vim 7.4+, vim 9, and nvim. No
" vim9script lock-in.

if exists('g:loaded_powerliners')
  finish
endif
let g:loaded_powerliners = 1

" Default to `powerline` from $PATH; user can override before
" sourcing. The `powerline` client speaks unix socket to
" powerline-daemon when present and falls back to one-shot
" powerline-render when no daemon is running.
if !exists('g:powerliners_binary')
  let g:powerliners_binary = 'powerline'
endif

" Per-request renderer args. Each entry becomes
" `-r key=value`. `mode`, `bufnr`, `winnr`, `buf_name` match
" upstream's `powerline.bindings.vim` keys so theme JSON written for
" upstream renders identically here.
function! s:RendererArgs() abort
  let l:args = []
  call add(l:args, '-r')
  call add(l:args, 'mode=' . mode(1))
  call add(l:args, '-r')
  call add(l:args, 'bufnr=' . bufnr('%'))
  call add(l:args, '-r')
  call add(l:args, 'winnr=' . winnr())
  let l:bn = expand('%:p')
  if !empty(l:bn)
    call add(l:args, '-r')
    call add(l:args, 'buf_name=' . l:bn)
  endif
  return l:args
endfunction

" Build the shell command using shellescape for every arg so paths
" with spaces don't fragment.
function! s:BuildCmd(side) abort
  let l:cmd = shellescape(g:powerliners_binary) . ' vim ' . shellescape(a:side)
  for l:arg in s:RendererArgs()
    let l:cmd .= ' ' . shellescape(l:arg)
  endfor
  return l:cmd
endfunction

" Run the binary once and split the response into (hi commands,
" statusline). Returns a list `[commands_string, statusline_string]`.
function! s:Render(side) abort
  let l:out = system(s:BuildCmd(a:side))
  if v:shell_error != 0
    return ['', '']
  endif
  " Split into [hi_block, statusline]. The wire format uses an empty
  " line (`\n\n`) as the delimiter; find the LAST occurrence so a
  " statusline that happens to contain a stray `\n\n` doesn't
  " truncate the hi block (unlikely but defensive).
  let l:idx = strridx(l:out, "\n\n")
  if l:idx < 0
    return ['', substitute(l:out, '\n\+$', '', '')]
  endif
  let l:hi_block = strpart(l:out, 0, l:idx)
  let l:statusline = substitute(strpart(l:out, l:idx + 2), '\n\+$', '', '')
  return [l:hi_block, l:statusline]
endfunction

" Apply rendered output. Execute each `hi` line silently and assign
" the statusline. The renderer guarantees valid `hi` syntax so the
" try/catch is purely defensive.
function! s:Apply(side) abort
  let l:result = s:Render(a:side)
  if empty(l:result[0]) && empty(l:result[1])
    return
  endif
  if !empty(l:result[0])
    for l:line in split(l:result[0], "\n")
      let l:l = substitute(l:line, '^\s\+\|\s\+$', '', 'g')
      if !empty(l:l)
        try
          silent execute l:l
        catch
        endtry
      endif
    endfor
  endif
  let &l:statusline = l:result[1]
endfunction

" Public entry — called from `:PowerlinersRefresh` and autocmds.
function! g:PowerlinersRefresh() abort
  call s:Apply('left')
endfunction

" laststatus=2 forces vim to always render the statusline; without
" it single-window sessions hide it entirely.
set laststatus=2

augroup powerliners
  autocmd!
  " Re-render on every event vim emits when the statusline could
  " need to change. `ModeChanged` (vim ≥8.2.2871) is the right hook
  " for mode transitions; the CursorMoved fallback covers ancient
  " vim where ModeChanged isn't available.
  autocmd VimEnter,WinEnter,BufWinEnter,BufEnter,TabEnter * call g:PowerlinersRefresh()
  if exists('##ModeChanged')
    autocmd ModeChanged * call g:PowerlinersRefresh()
  else
    autocmd CursorMoved,CursorMovedI * call g:PowerlinersRefresh()
  endif
augroup END

command! PowerlinersRefresh call g:PowerlinersRefresh()
