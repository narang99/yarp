Every platform has different search semantics on how to "find" libraries. We need to replicate it as faithfully as we can.  

# Why do we need searching?
The first approach was to use a very simple semantic. Get the list of all loaded libraries in the python process using `dyld_image_count`. For every library, use the leaf name of the library to be searched, search the libraries in `dyld_image_count`. If it exists, we good, use that. Else error out.  
The problem however, is that `dyld_image_count` only returns the real paths of the libraries which are loaded.  
A `dlopen` call can be done on a search term, which might be a symlink to the real file. As an example `pango-1.0` points to some lib `<absolute-path-to>/pango-1.0.dylib`. This path is actually a symlink to `<absolute-path-to>/pango-2.0.dylib`, we only get this library in the `dyld_image_count` list.  
The symlinks are arbitrary (as above). Its impossible to "guess" some heuristics to find symlinks.  
We would need `pango-1.0.dylib` in the library path of the final packaged application.  
If we just use real paths from `dyld_image_count`, its not going to work.  


So we do the search ourselves too. The search needs to be exactly the same as dyld to make this work.   
Both the python lib and the rust binary need to implement the search.  
- individual python FFI libraries might augment the search in arbitrary ways, we need to faithfully recreate that too in the python library code
- rust code basically includes all dylibs in a greedy fashion, in this case it also needs to resolve the search path for LC_LOAD commands

# Search Algorithm: Mac (dyld)
any library which provides dlopen utilities, has some semantics for searching paths before calling the syscall dlopen
dlopen itself has some semantics, described in its man page. 

dlopen uses this search order:
- if path does not contain any slash
  - DYLD_LIBRARY_PATH
  - RPATH of executable (sys.executable in our case). or the library (if its as a load command inside dylib)
  - current directory (this is simply Path.cwd())
  - DYLD_FALLBACK_LIBRARY_PATH
  - /usr/local/lib
  - /usr/lib
- if path looks like a framework
  - DYLD_FRAMEWORK_PATH
  - use path as is, if its relative, append to cwd and make absolute
  - DYLD_FALLBACK_FRAMEWORK_PATH
  - /Library/Frameworks
  - /System/Library/Frameworks
- if path has a slash but is not a framework path
  - DYLD_LIBRARY_PATH (use the leaf of the path)
  - use path as is, if its relative, append to cwd to make absolute
  - DYLD_FALLBACK_LIBRARY_PATH
  - /usr/local/lib
  - /usr/lib

we need to do this searching ourselves. The search might be actually augmented by individual FFI libraries (with their own extra semantics)
Our python code needs to do this searching individually for each FFI library (encode their semantics, fake it basically)
depending on library version, we might need to do more. The easiest way is to call private functions if they exist, and then use our fallback in the worst case

The rust code also needs to implement these search semantics, for parsing dylibs. 
When we are parsing dylibs, we might need to implement the search above, the current search is not enough

`ctypes` has this exact algorithm encoded in it, we use it as our fallback (`find_library`).  