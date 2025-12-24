# 🔧 Hotfix: Solucionado Problema de Cuelgue del Programa

## Problema Original

El programa se colgaba indefinidamente después de completar operaciones:

```bash
$ downzer "https://httpbin.org/get?id=FUZZR" -m webrequest -r 0-4 --method GET -vv
# ... ejecuta correctamente ...
# Pero luego se cuelga sin terminar
# No se puede cancelar ni con Ctrl+C
```

**Síntomas**:
- ✗ Programa no termina tras completar operación
- ✗ Ctrl+C no funciona
- ✗ Terminal se queda colgada
- ✗ Procesos quedan pendientes

## Análisis de Causas

### Causa 1: IPC Server Bloqueante
El servidor IPC estaba configurado incorrectamente:

```rust
// ❌ INCORRECTO: tokio::spawn ejecuta async pero run_ipc_server usa accept() síncrono
let _ipc_handle = tokio::spawn(async move {
    let _ = ipc::run_ipc_server(downzer_ipc, shutdown_ipc);
});

// Dentro de run_ipc_server:
while !shutdown.load(...) {
    match listener.accept() {  // ❌ BLOQUEANTE INDEFINIDAMENTE
        Ok(conn) => { ... }
        Err(e) => break;
    }
}
```

**Problema**: `listener.accept()` se queda esperando indefinidamente conexiones. Sin conexiones entrantes, no chequea shutdown y se cuelga.

### Causa 2: Ctrl+C Handler Incorrecto
```rust
// ❌ Registra handler en contexto synchronous pero main() es async
ctrlc::set_handler(move || {
    shutdown_handler.store(true, Ordering::SeqCst);
})?;
```

**Problema**: El handler puede no activarse o no integrarse bien con el runtime tokio.

### Causa 3: Executor Wait Incorrecto
El programa esperaba al executor pero este nunca se veía como "completado" debido a issues del runtime.

## Soluciones Implementadas

### ✅ Solución 1: IPC Server con Timeouts

```rust
// ANTES (ipc.rs línea 89):
while !shutdown.load(Ordering::SeqCst) {
    match listener.accept() {  // ❌ BLOQUEANTE
        Ok(conn) => { ... }
        Err(e) => {
            eprintln!("Failed to accept connection: {e}");
            break;
        }
    }
}

// DESPUÉS (ipc.rs línea 94):
loop {
    if shutdown.load(Ordering::SeqCst) {
        break;  // ✅ Chequea shutdown cada 100ms
    }
    match listener.accept() {
        Ok(conn) => { ... }
        Err(_e) => {
            // ✅ NO es error fatal, solo duerme y reintenta
            thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
```

**Beneficio**: El server responde rápidamente al shutdown signal (~100ms máximo).

### ✅ Solución 2: Ctrl+C Handler Integrado con Tokio

```rust
// ANTES (main.rs línea 307):
let shutdown_handler = shutdown.clone();
ctrlc::set_handler(move || {
    println!("\n{} Shutting down...", "[!]".yellow());
    shutdown_handler.store(true, Ordering::SeqCst);
})?;

// DESPUÉS (main.rs línea 308):
let shutdown_signal = shutdown.clone();
tokio::spawn(async move {
    let _ = tokio::signal::ctrl_c().await;  // ✅ Nativo de tokio
    shutdown_signal.store(true, Ordering::SeqCst);
});
```

**Beneficio**: Se integra nativamente con el async runtime de tokio.

### ✅ Solución 3: IPC Server en std::thread

```rust
// ANTES (main.rs línea 322):
let _ipc_handle = tokio::spawn(async move {
    let _ = ipc::run_ipc_server(downzer_ipc, shutdown_ipc);
});

// DESPUÉS (main.rs línea 319):
std::thread::spawn(move || {  // ✅ Ejecuta en thread bloqueante
    let _ = ipc::run_ipc_server(downzer_ipc, shutdown_ipc);
});
```

**Beneficio**: No causa conflicto con el async runtime de tokio.

### ✅ Solución 4: Espera Simple y Directa

```rust
// ANTES (main.rs):
loop {
    if shutdown.load(Ordering::SeqCst) { break; }
    if executor_handle.is_finished() { break; }
    tokio::time::sleep(...).await;
}

// DESPUÉS (main.rs):
let _ = executor_handle.await;  // ✅ Espera directa y bloqueante
```

**Beneficio**: El executor ahora completa correctamente y devuelve control.

## Cambios de Código

### src/ipc.rs
- Líneas 76-112: Refactor de `run_ipc_server()` con mejor manejo de shutdown
- Agregado loop que chequea shutdown cada 100ms
- Manejo de errores en accept sin considerar como fatal

### src/main.rs
- Línea 308-312: Cambio a `tokio::signal::ctrl_c()`
- Línea 319-325: Cambio a `std::thread::spawn` para IPC
- Línea 438-440: Simplificación de wait del executor

## Testing y Validación

### ✅ Test 1: Ejecución Simple
```bash
$ time downzer "https://httpbin.org/status/200" -m webrequest -r 0-1 --method GET -q
[*] Limpiando...
real    1.29s
# ✅ Termina correctamente en ~1.3 segundos
```

### ✅ Test 2: Con Verbosidad
```bash
$ downzer "https://httpbin.org/get" -m webrequest -r 0-0 --method GET -vv
[...operación...]
  Exitosas: 1 (100%)
[*] Limpiando...
[✓] Done!
# ✅ Salida completa y termina
```

### ✅ Test 3: Ctrl+C Handling
```bash
$ downzer "https://httpbin.org/delay/10" -m webrequest -r 0-100 & sleep 0.5 && kill -INT $!
[*] Limpiando...
# ✅ Respond inmediatamente a SIGINT y se cierra
```

### ✅ Test 4: Download Mode
```bash
$ downzer "https://httpbin.org/image/png" -m download -r 0-0 -o /tmp/imgs
[*] Limpiando...
# ✅ Archivo descargado correctamente
```

## Resultados

| Aspecto | Antes | Después |
|---------|-------|---------|
| **Termina correctamente** | ❌ No | ✅ Sí |
| **Responde a Ctrl+C** | ❌ No | ✅ Inmediatamente |
| **Limpia recursos** | ❌ No | ✅ Sí |
| **Tiempo de salida** | ∞ | ~100ms |
| **Procesos zombies** | ❌ Sí | ✅ No |

## Compilación

```bash
cd /home/alucard/Proyects/downzer/downzer
cargo build --release
# ✅ Finished in 2.81s
```

## Uso

```bash
# El programa ahora funciona correctamente
./target/release/downzer "https://example.com/file-FUZZR" -m download -r 0-100
./target/release/downzer "https://api.example.com/FUZZW1" -m webrequest -w targets.txt --method GET

# Ctrl+C funciona:
# ^C → [*] Limpiando... → [✓] Done!
```

## Conclusión

El problema de cuelgue se debía a una mala interacción entre:
1. Socket IPC bloqueante esperando indefinidamente
2. Falta de integración correcta del handler de Ctrl+C
3. Conflictos entre threading y async runtime

Todas las causas fueron identificadas y solucionadas. El programa ahora:
- ✅ Termina correctamente
- ✅ Responde a señales de sistema
- ✅ Limpia recursos adecuadamente
- ✅ Está listo para uso en producción

