basics to revisit in the end


- Most important: follow linker as closely as possible. python and rust code: how they interact in this?
  - install names vs path names
  - loading in the exact order as the linker did
  - what should be the name in reals?
  - versioning? how does it affect stuff? is `libHello.so` same as `libHello.so.1`?
  - naming conventions of binaries
  - which patterns should be searched?
- Fat binaries, how to handle them?
- Error messaging (keep an error enum which we downcast in the end for CLI based error enum printing, like E001 is a variant kinda thing)
- number of clones in the application, add more lifetime specifiers for read only stuff at least
- in exit handler, import everything, to get as many packages as we can