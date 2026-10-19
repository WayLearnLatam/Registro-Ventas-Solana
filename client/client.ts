//////////////////// Imports ////////////////////
import { PublicKey } from "@solana/web3.js";

////////////////// Constantes ////////////////////
const nombre_negocio = "La tiendita WayLearn"; // Nombre del punto de venta
const owner = pg.wallet.publicKey; // Wallet

//////////////////// Client Test Logs ////////////////////
console.log("My address:", owner.toString()); // Ver el address
const balance = await pg.connection.getBalance(owner);
console.log(`My balance: ${balance / web3.LAMPORTS_PER_SOL} SOL`); // Ver la cantidad de tokens de solana

//////////////////// FUNCIONES ////////////////////

//////////////////// OBTENER PDAs ////////////////////
/*
Un PDA representa una cuenta que es controlada por un programa (smart contract), y una de sus principales caracteristicas es no contar
con una clave privada con la cual firmar al momento de realizar alguna transaccion (transferencia, escritura o modificacion de un dato)
dentro del contrato. En su lugar, emplea direcciones generadas deterministicamente, es decir, recreables a partir de semillas.

Es por ello que para llamar desde el front una funcion del Solana Program desplegado es necesario contar con las semillas en su orden y tipo
correspondiente.

En esta version SOLO existe una PDA: la del registro. Los clientes y sus ventas son structs guardados dentro de un vector
en esa misma cuenta, por lo que no tienen direccion propia.
*/
//////////////////// Registro ////////////////////
function pdaRegistro() {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("registro"), // Semilla 1: b"registro"
      owner.toBuffer(), // Semilla 2: wallet -> Pubkey
    ],
    pg.PROGRAM_ID // Program ID: Siempre va al final
  );
}

//////////////////// Crear Registro ////////////////////
// Para crear el registro solo es necesario el nombre que tendra el negocio
async function crearRegistro(nombre_negocio) {
  const [pda_registro] = pdaRegistro(); // Primero se obtiene la cuenta del registro

  const txHash = await pg.program.methods // mediante la libreria pg (solana playground) se acceden a los metodos del programa
    .crearRegistro(nombre_negocio) // crear_registro -> en el IDL pasa a camelCase
    .accounts({
      // Se agregan las cuentas de las que depende (Contexto del struct NuevoRegistro)
      owner: owner,
      registro: pda_registro,
    })
    .rpc();

  console.log("txHash: ", txHash);
}

//////////////////// Registrar Venta ////////////////////
/*
La llamada es identica para un cliente nuevo y para uno existente: mismas cuentas, misma
instruccion. Lo unico que cambia es el primer argumento, porque aqui no se crea ninguna cuenta.

    * id_cliente = 0  -> el programa crea el cliente y le asigna el id
    * id_cliente != 0 -> el programa lo busca en el vector

monto es u32 y id_cliente u16: Anchor acepta numeros de JS hasta 32 bits. A partir de u64
hay que usar new BN(valor).
*/
async function registrarVenta(id_cliente, sku, descripcion, monto) {
  const [pda_registro] = pdaRegistro(); // se obtiene la cuenta del registro

  const txHash = await pg.program.methods
    .registrarVenta(id_cliente, sku, descripcion, monto) // registrar_venta
    .accounts({
      // cuentas del contexto (ModificarVenta)
      owner: owner,
      registro: pda_registro,
    })
    .rpc();

  console.log("txHash: ", txHash);
}

//////////////////// Hacer Devolucion ////////////////////
// Para devolver una venta se necesita el id del cliente y el SKU del producto
async function hacerDevolucion(id_cliente, sku) {
  const [pda_registro] = pdaRegistro(); // se obtiene la cuenta del registro

  const txHash = await pg.program.methods
    .hacerDevolucion(id_cliente, sku) // hacer_devolucion
    .accounts({
      // cuentas del contexto
      owner: owner,
      registro: pda_registro,
    })
    .rpc();

  console.log("txHash: ", txHash);
}

//////////////////// Cambiar Monto ////////////////////
// Para corregir el importe se necesita el id del cliente, el SKU y el nuevo monto
async function cambiarMonto(id_cliente, sku, nuevo_monto) {
  const [pda_registro] = pdaRegistro(); // se obtiene la cuenta del registro

  const txHash = await pg.program.methods
    .cambiarMonto(id_cliente, sku, nuevo_monto) // cambiar_monto
    .accounts({
      // cuentas del contexto
      owner: owner,
      registro: pda_registro,
    })
    .rpc();

  console.log("txHash: ", txHash);
}

//////////////////// Cantidad de Compras ////////////////////
// El resultado NO llega aqui: msg! escribe en los logs de la transaccion, no en el valor de
// retorno. Para ver el numero hay que usar "solana confirm -v <txHash>", o leer la cuenta
// directamente con verClientes().
async function cantidadCompras(id_cliente) {
  const [pda_registro] = pdaRegistro(); // se obtiene la cuenta del registro

  const txHash = await pg.program.methods
    .cantidadCompras(id_cliente) // cantidad_compras
    .accounts({
      // cuentas del contexto (ConsultarRegistro)
      owner: owner,
      registro: pda_registro,
    })
    .rpc();

  console.log("txHash: ", txHash);
  console.log("Para ver el resultado: solana confirm -v", txHash);
}

//////////////////// Ver Clientes ////////////////////
/*
Lectura completa del registro: clientes y, dentro de cada uno, su historial de ventas.

Basta UN SOLO fetch. Como los clientes y sus ventas son structs guardados dentro de la cuenta
registro, al traer esa cuenta ya viene todo. Esa es la ventaja de este modelo, y tambien su
limite: toda esa informacion tuvo que caber en una cuenta de tamaño fijo.

Pasos:

1. Determinar el PDA del registro
2. Traer la cuenta con fetch
3. Recorrer el vector de clientes
4. Por cada cliente, recorrer su historial
5. Mostrarlo con console.log
*/
async function verClientes() {
  const [pda_registro] = pdaRegistro(); // se obtiene la cuenta del registro

  try {
    // Se accede a los datos de la cuenta (registro). Un solo fetch trae TODO.
    const registroAccount = await pg.program.account.registro.fetch(
      pda_registro
    );

    console.log("Negocio:", registroAccount.nombreNegocio); // En el IDL nombre_negocio pasa a nombreNegocio
    console.log("Ultimo id asignado:", registroAccount.idCounter);

    // Mediante el .length se obtiene el tamaño del vector de clientes
    const numero_clientes = registroAccount.clientes.length;

    // Se verifica si hay clientes en el vector
    if (!registroAccount.clientes || numero_clientes === 0) {
      console.log("Registro vacío, aún no hay clientes");
      return;
    }

    console.log("Cantidad de clientes:", numero_clientes);

    // Se itera cada cliente del vector
    for (let i = 0; i < numero_clientes; i++) {
      const cliente = registroAccount.clientes[i];

      console.log(
        `\nCliente #${i + 1}: \n * Id: ${cliente.idCliente} \n * Compras: ${
          cliente.compras
        } \n * Ventas en historial: ${cliente.historial.length}`
      );

      // Su historial ya esta en memoria: no hace falta otro fetch
      for (let j = 0; j < cliente.historial.length; j++) {
        const venta = cliente.historial[j];

        console.log(
          `   Venta #${j + 1}: SKU ${venta.sku} | ${venta.descripcion} | $${
            venta.monto
          }`
        );
      }
    }
  } catch (error) {
    console.error("Error viendo clientes:", error);

    // Debugging adicional
    if (error.message) {
      console.error("Mensaje de error:", error.message);
    }
    if (error.logs) {
      console.error("Logs del programa:", error.logs);
    }
  }
}

// crearRegistro(nombre_negocio);
// registrarVenta(0, "Pan", "Pan bimbo", 48); // id 0 -> crea cliente nuevo
// registrarVenta(0, "Pan", "Pan bimbo", 48); // id 0 -> crea cliente nuevo
// registrarVenta(1, "Cafe", "Cafe del oxxo", 38);    // id 1 -> cliente existente
// cantidadCompras(1);
// cantidadCompras(2);
// cambiarMonto(1, "Pan", 50);
// cantidadCompras(1);
// hacerDevolucion(1, "Cafe");
// cantidadCompras(1);
// verClientes();

// solana confirm -v <txHash>
