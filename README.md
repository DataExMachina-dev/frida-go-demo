# frida-go-demo

This repo contains some programs that do interesting things involving go and frida.
Everything in this repo assumes a linux machine running x86_64.

## Playing with the `frida` tool

### Hooking a cgo function, all good

The first thing to demonstrate is that hooking a cgo function "just works" using
the high-level frida tools. You can see this by running the `cgo_program` and
then using the frida tools to hook the `println_from_c` function.

This example assumes you're in the `cgo_program` directory.

In two terminals do the following and observe the hooking working:

```bash
go build ./
./cgo_program >/dev/null
```

Either you'll need to set up your ptrace_scope to 0 or use root:

```bash
frida cgo_program -l frida_intercept_cgo_call.js
```

### Hooking a go function and crashing the program

This step demonstrates that there's something wrong with hooking go functions
using the high-level frida tools (note that this is not the primary focus of
this document, but it's worth seeing).

In two terminals do the following and observe the hooking working:

```bash
go build .
./cgo_program >/dev/null
```

Either you'll need to set up your ptrace_scope to 0 or use root:

```bash
frida cgo_program -l frida_intercept_go_call.js
```

## Dropping down to frida-gum and frida-core

Now let's drop down and start playing with injecting into a go program using the
lower-level bindings to frida accessed through rust.

In this example we build a library that we'll inject using frida's injector and
then in that library we'll intercept a function call using frida-gum. The thing
that we want to demonstrate is how sensitive the go program is to stack space.
The go program's have a stack guard that guarantees 800 bytes of safe stack
space for use when hooking a normal go program. It can be up to 928 bytes, but
don't worry about that.

The go program in question is highly recursive so it ends up growing its stack
regularly and using irregular stack sizes. This will be important to demonstrate
the problems posed by go for frida invocation listeners and friends.

For this let's build the dylib we'll be injecting:

```bash
cargo build --lib frida-lib-to-inject --release
```

### Run the injector using the stack measurement action

In a shell build and start the go program:

```bash
cd go_program
go build .
./go_program
```

In another shell, run the injector, first with just the `measure-stack` action:

```bash
cargo run --bin frida-injector -- \
 --target $(pgrep go_program) \
 --function 'main.recurseA' \
 --lib-to-inject target/release/libfrida_lib_to_inject.so \
 --action measure-stack
```

After this you should see the go program start to print out the following:

```
busyLoop starting
running
total_size: 848
total_size: 840
total_size: 840
total_size: 840
```

### Run the injector using the do-more-stuff action (crash)

Now restart the go program in the first shell. In the second shell, change the action

```bash
cargo run --bin frida-injector -- \
 --target $(pgrep go_program) \
 --function 'main.recurseA' \
 --lib-to-inject target/release/libfrida_lib_to_inject.so \
 --action do-more-stuff
```

Observe the go program crashing like this:

```
busyLoop starting
running
total_size: 920
runtime: pointer 0xc000541fe0 to unused region of span span.base()=0xc0004fc000 span.limit=0xc0004fdfe0 span.state=1
runtime: found in object at *(0xc000457180+0x80)
object=0xc000457180 s.base()=0xc000456000 s.limit=0xc000457f80 s.spanclass=48 s.elemsize=448 s.state=mSpanInUse
 *(object+0) = 0x0
 *(object+8) = 0x0
 *(object+16) = 0x0
 *(object+24) = 0xffffffffffffffff
 *(object+32) = 0x0
 *(object+40) = 0x0
 *(object+48) = 0x0
 *(object+56) = 0xc000541fb8
 *(object+64) = 0x44149d
 *(object+72) = 0xc000457180
 *(object+80) = 0x0
 *(object+88) = 0x0
 *(object+96) = 0x0
 *(object+104) = 0xc000541fd0
 *(object+112) = 0x0
 *(object+120) = 0x0
 *(object+128) = 0xc000541fe0 <==
 *(object+136) = 0x0
 *(object+144) = 0x6
 *(object+152) = 0x8f50c
 *(object+160) = 0xc00044ce00
 *(object+168) = 0x0
 *(object+176) = 0x1000000000000
 *(object+184) = 0xd000000000000000
 *(object+192) = 0x0
 *(object+200) = 0x0
 *(object+208) = 0x0
 *(object+216) = 0x0
 *(object+224) = 0x0
 *(object+232) = 0x0
 *(object+240) = 0x0
 *(object+248) = 0x0
 *(object+256) = 0x0
 *(object+264) = 0x0
 *(object+272) = 0x1
 *(object+280) = 0x496479
 *(object+288) = 0x0
 *(object+296) = 0x496560
 *(object+304) = 0x0
 *(object+312) = 0x0
 *(object+320) = 0x0
 *(object+328) = 0x0
 *(object+336) = 0x0
 *(object+344) = 0x0
 *(object+352) = 0x0
 *(object+360) = 0x0
 *(object+368) = 0x0
 *(object+376) = 0x0
 *(object+384) = 0x0
 *(object+392) = 0x0
 *(object+400) = 0x0
 *(object+408) = 0x0
 *(object+416) = 0x0
 *(object+424) = 0x0
 *(object+432) = 0x0
 *(object+440) = 0x0
fatal error: found bad pointer in Go heap (incorrect use of unsafe or cgo?)
```

### Explanation

As we've mentioned before, go stacks are on the heap. The way frida works (as
far as we've observed) for its invocation listeners is to inject a jump
instruction to a trampoline which spills the context, sets up some data
structures and invokes the listener. The code to spill the context in frida uses
just shy of 800 bytes worth of stack in our experimentation. 512 of these bytes
are in the [fxsave](https://www.felixcloutier.com/x86/fxsave) instruction. This
leaves very little room for the listener to do things without corrupting the
stack.

The trampoline logic is _almost_ enough to let a probe safely run. Perhaps if
Frida could shave off a few 10s of bytes in its trampoline logic then the
listener itself could deal with switching stacks. We think that would be a bad
answer; it doesn't leave much room for comfort given issues like
https://github.com/golang/go/issues/51256 in go. Ideally we'd find a way to
rearrange frida so that it can spill the context and execute the listener on a
different stack so that the injected program can have at least kilobytes of
stack space.

The 800 bytes of stack guard comes from
[here](https://github.com/golang/go/blob/959e65c41cf9aebb5af72466023ac66b01baf9e9/src/internal/abi/stack.go#L8-L14).
Go reserves this many bytes of available stack space for some of its special
internal runtime functions.

We confirmed that this really is the cause of the corruption with more certainty
by modifying the go runtime to reserve more stack guard and saw that a
previously crashing program stopped crashing. We did this in two ways:

1. We modified `stackGuard`
   [here](https://github.com/golang/go/blob/959e65c41cf9aebb5af72466023ac66b01baf9e9/src/runtime/stack.go#L99)
   to be larger
2. We modified the stack growth checking logic to grow more eagerly
   [here](https://github.com/golang/go/blob/dc164eade1efd819d54dabf121bec0386019421b/src/cmd/internal/obj/x86/obj6.go#L1120),
   note that this also requires eliminating the `StackSmall` first arm of the
   containing `if`.

## Other notes

The `go_program` imports `net` so that it will not be statically linked. As
we've discussed before, frida doesn't seem to work correctly to inject a dynamic
loader on statically linked go binaries.
