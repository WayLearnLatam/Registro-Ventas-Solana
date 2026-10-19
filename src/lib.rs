use anchor_lang::prelude::*;
// ID del Solana Program, este espacio se llena automaticamente al hacer el "build"
declare_id!("");

#[program] // Macro que convierte codigo de Rust a Solana.
pub mod registro_ventas {
    use super::*; // Importa todos los structs y enums definidos fuera del modulo

    //////////////////////////// Instruccion: Crear Registro /////////////////////////////////////
    /*
    Permite la creacion de una PDA (Program Derived Address), un tipo especial de cuenta en Solana
    que permite prescindir del uso de llaves privadas para la firma de transacciones.

    Esta cuenta contendra el objeto (struct) de tipo Registro, donde se almacenaran TODOS los
    clientes del negocio y, dentro de cada cliente, TODO su historial de ventas.
    La creacion de la PDA depende de 3 cosas:
        * Wallet address (owner)
        * Program ID
        * string representativo, regularmente relacionado con el nombre del proyecto

    La explicacion de esto continua en el struct NuevoRegistro

    Parametros de entrada:
        * nombre_negocio -> nombre del punto de venta -> tipo String
     */
    pub fn crear_registro(context: Context<NuevoRegistro>, nombre_negocio: String) -> Result<()> {
        // "Context" siempre suele ir como primer parametro, ya que permite acceder al objeto o
        // cuenta con el que queremos interactuar.
        let owner_id = context.accounts.owner.key(); // Accedemos al wallet address del caller

        let clientes = Vec::new(); // Crea un vector vacio: el negocio nace sin clientes
        let id_counter: u16 = 0; // Contador incremental, asigna un id unico a cada cliente nuevo

        // Creamos un Struct de tipo Registro y lo guardamos directamente en la cuenta
        context.accounts.registro.set_inner(Registro {
            owner: owner_id,
            nombre_negocio: nombre_negocio.clone(), // .clone() porque el String se vuelve a usar en el msg!
            clientes,
            id_counter,
        });

        msg!("Registro: {} creado exitosamente!", nombre_negocio); // Print de verificacion

        Ok(()) // Representa una transaccion exitosa
    }

    //////////////////////////// Instruccion: Registrar Venta /////////////////////////////////////
    /*
    Registra una venta. Tiene dos comportamientos segun el valor de id_cliente:
        * id_cliente == 0 -> se interpreta como "cliente nuevo": se incrementa el contador,
          se crea un struct Cliente y la venta queda como primer elemento de su historial.
        * id_cliente != 0 -> se busca al cliente dentro del vector y se le agrega la venta.

    Nota importante: aqui NO se crea ninguna cuenta nueva. Todo (clientes y ventas) vive dentro
    de la unica cuenta Registro. Por eso una sola instruccion puede atender ambos casos.

    Parametros de entrada:
        * id_cliente  -> id del cliente, 0 = cliente nuevo -> u16
        * sku         -> codigo del producto vendido       -> String
        * descripcion -> descripcion de la venta           -> String
        * monto       -> importe de la venta               -> u32
     */
    pub fn registrar_venta(
        context: Context<ModificarVenta>,
        id_cliente: u16,
        sku: String,
        descripcion: String,
        monto: u32,
    ) -> Result<()> {
        require!(
            // Medida de seguridad para identificar que SOLO el owner del registro haga cambios en el
            context.accounts.registro.owner == context.accounts.owner.key(), // true -> continua, false -> error
            Errores::NoOwner // Codigo de error, ver enum Errores
        );

        let registro = &mut context.accounts.registro; // Referencia mutable a la cuenta completa

        let venta = Venta {
            // Creacion de un struct tipo Venta
            sku,
            descripcion,
            monto,
        };

        if id_cliente == 0 {
            // ---- Caso A: cliente nuevo ----
            registro.id_counter += 1; // El contador se guarda en la cuenta
            msg!(
                "Cliente creado con id: {}, del articulo {} registrada por un total de {}",
                registro.id_counter,
                venta.sku,
                venta.monto
            );
            let id = registro.id_counter;
            registro.clientes.push(Cliente {
                id_cliente: id, // El programa asigna el id, no el frontend
                compras: 1,
                historial: vec![venta], // vec![x] inicializa el vector CON un elemento, a diferencia de Vec::new()
            });

            return Ok(()); // Transaccion exitosa, sale sin recorrer el vector
        } else {
            // ---- Caso B: cliente existente ----
            for i in 0..registro.clientes.len() {
                // Se itera mediante el indice todo el vector en busca del cliente
                if registro.clientes[i].id_cliente == id_cliente {
                    msg!(
                        "Compra del articulo {} registrada por un total de {}",
                        venta.sku,
                        venta.monto
                    );
                    registro.clientes[i].historial.push(venta); // Agrega la venta al historial de ese cliente
                    registro.clientes[i].compras += 1; // Mantiene el contador de compras sincronizado

                    return Ok(()); // Transaccion exitosa
                }
            }
        }

        // Si el for termino sin encontrarlo, el cliente no existe
        msg!("Cliente con id: {} no encontrado!", id_cliente);
        Err(Errores::NoUser.into()) // Transaccion fallida: revierte TODOS los cambios anteriores
    }

    //////////////////////////// Instruccion: Hacer Devolucion /////////////////////////////////////
    /*
    Elimina una venta del historial de un cliente a partir de su SKU.
    Error si el cliente no existe, error si su historial esta vacio, error si el SKU no aparece.

    Nota: la venta se BORRA del historial, no se marca como devuelta. Para una demo esta bien,
    pero en una aplicacion real se perderia la trazabilidad contable de la operacion.

    Parametros de entrada:
        * id_cliente -> id del cliente que devuelve -> u16
        * sku        -> codigo del producto devuelto -> String
     */
    pub fn hacer_devolucion(
        context: Context<ModificarVenta>,
        id_cliente: u16,
        sku: String,
    ) -> Result<()> {
        require!(
            // Medida de seguridad
            context.accounts.registro.owner == context.accounts.owner.key(),
            Errores::NoOwner
        );
        let clientes = &mut context.accounts.registro.clientes; // Referencia mutable al vector de clientes

        for i in 0..clientes.len() {
            // Bucle externo: busca al cliente
            if clientes[i].id_cliente == id_cliente {
                // Si su historial esta vacio no hay nada que devolver. Este require tambien evita
                // que la resta de "compras" mas abajo llegue a hacer underflow.
                require!(!clientes[i].historial.is_empty(), Errores::SinCompras);

                for j in 0..clientes[i].historial.len() {
                    // Bucle interno: busca la venta por SKU
                    if clientes[i].historial[j].sku == sku {
                        clientes[i].historial.remove(j); // remove() desplaza los elementos posteriores
                        clientes[i].compras -= 1;
                        msg!("Devolucion realizada exitosamente");
                        return Ok(()); // Transaccion exitosa
                    }
                }

                // El cliente SI existe, pero no compro ese SKU: error distinto al de cliente inexistente
                return Err(Errores::NoSKU.into());
            }
        }
        Err(Errores::NoUser.into()) // Transaccion fallida: nunca encontro al cliente
    }

    //////////////////////////// Instruccion: Cambiar Monto /////////////////////////////////////
    /*
    Corrige el importe de una venta ya registrada, localizandola por cliente y SKU.

    Parametros de entrada:
        * id_cliente   -> id del cliente             -> u16
        * sku          -> codigo del producto        -> String
        * nuevo_monto  -> importe corregido          -> u32
     */
    pub fn cambiar_monto(
        context: Context<ModificarVenta>,
        id_cliente: u16,
        sku: String,
        nuevo_monto: u32,
    ) -> Result<()> {
        require!(
            // Medida de seguridad
            context.accounts.registro.owner == context.accounts.owner.key(),
            Errores::NoOwner
        );

        let clientes = &mut context.accounts.registro.clientes;

        for i in 0..clientes.len() {
            if clientes[i].id_cliente == id_cliente {
                for j in 0..clientes[i].historial.len() {
                    if clientes[i].historial[j].sku == sku {
                        let viejo_monto = clientes[i].historial[j].monto;
                        clientes[i].historial[j].monto = nuevo_monto; // Modificacion directa del campo
                        msg!(
                            "Monto actualizado existosamente, paso de {} a {}.",
                            viejo_monto,
                            nuevo_monto
                        );
                        return Ok(());
                    }
                }
                // Cliente encontrado pero SKU no: el return evita seguir buscando en los demas clientes
                return Err(Errores::NoSKU.into());
            }
        }
        Err(Errores::NoUser.into())
    }

    //////////////////////////// Instruccion: Cantidad de Compras /////////////////////////////////////
    /*
    Muestra en el log de la transaccion el total de compras de un cliente.

    IMPORTANTE para la clase: una instruccion on-chain NO devuelve valores al frontend.
    msg! solo escribe en los logs de la transaccion. Para LEER datos, el cliente hace
    program.account.registro.fetch(pda) y calcula lo que necesite.
    Regla general: la cuenta es la API de lectura, las instrucciones son la API de escritura.

    Parametros de entrada:
        * id_cliente -> id del cliente a consultar -> u16
     */
    pub fn cantidad_compras(context: Context<ModificarVenta>, id_cliente: u16) -> Result<()> {
        require!(
            context.accounts.registro.owner == context.accounts.owner.key(),
            Errores::NoOwner
        );

        let clientes = &context.accounts.registro.clientes; // Referencia INMUTABLE: solo se lee

        for i in 0..clientes.len() {
            if clientes[i].id_cliente == id_cliente {
                msg!(
                    "El cliente tiene un total de compras de {}:",
                    clientes[i].compras
                );
                //  el uso de :#? permite la visualizacion completa del vector en el log
                msg!("Lista de productos comprados: {:#?}", clientes[i].historial);
                return Ok(());
            }
        }
        Err(Errores::NoUser.into())
    }
}

/*
Codigos de error
Todos los codigos se almacenan en un enum con la siguiente estructura:
#[msg("MENSAJE DE ERROR")] (dentro de las comillas)
NombreDelError, (En camel case)

Ojo con el nombre del enum: NO puede llamarse "Error", porque ese nombre ya lo ocupa
anchor_lang::prelude::Error (entra por el "use anchor_lang::prelude::*" de arriba).
*/
#[error_code]
pub enum Errores {
    #[msg("Error, no eres el owner del registro")]
    NoOwner,
    #[msg("Error, usuario no existe")]
    NoUser,
    #[msg("Error, sku no encontrado")]
    NoSKU,
    #[msg("Error, no hay compras en el historial")]
    SinCompras,
}

#[account] // Especifica que el struct es una cuenta que se almacenara en la blockchain
#[derive(InitSpace)] // Genera la constante INIT_SPACE y determina el espacio de almacenamiento necesario
pub struct Registro {
    owner: Pubkey, // Pubkey es un formato de llave publica de 32 bytes

    #[max_len(30)] // Cantidad maxima de caracteres del string: nombre_negocio
    nombre_negocio: String,

    #[max_len(10)] // Tamaño maximo del vector clientes -> ¡solo caben 10 clientes!
    clientes: Vec<Cliente>,

    id_counter: u16, // No requiere max_len: un u16 siempre ocupa 2 bytes
}

/*
Struct interno o secundario (No es una cuenta). Se define por derive y cuenta con los siguientes atributos:
    * AnchorSerialize   -> Permite guardar el struct en la cuenta
    * AnchorDeserialize -> Permite leer su contenido desde la cuenta
    * Clone             -> Para copiar su contenido o valores
    * InitSpace         -> Calcula el tamaño necesario para ser almacenado en la blockchain
    * PartialEq         -> Para usar sus valores y compararlos con "=="
    * Debug             -> Para mostrarlo en log con ":?" o ":#?"
*/
#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace, PartialEq, Debug)]
pub struct Cliente {
    id_cliente: u16,
    compras: u16,

    #[max_len(10)] // Tamaño maximo del historial -> solo 10 ventas POR CLIENTE
    historial: Vec<Venta>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace, PartialEq, Debug)]
pub struct Venta {
    #[max_len(5)] // Los SKU son codigos cortos
    sku: String,

    #[max_len(50)]
    descripcion: String,

    monto: u32, // No requiere max_len (numero de 32 bits)
}

// Creacion de los contextos para las instrucciones (funciones)
#[derive(Accounts)] // Especifica que este struct describe las cuentas que requiere determinada instruccion
pub struct NuevoRegistro<'info> {
    // contexto de la instruccion crear_registro
    #[account(mut)]
    pub owner: Signer<'info>, // El owner paga la transaccion, por eso es mut: cambia su balance

    #[account(
        init,                                          // Indica que al llamar la instruccion se creara una cuenta
        payer = owner,                                 // Quien paga el rent de la cuenta nueva
        space = Registro::INIT_SPACE + 8,              // Espacio requerido + 8 bytes del discriminador de Anchor
        seeds = [b"registro", owner.key().as_ref()],   // La cuenta es una PDA que depende de un string y el id del owner
        bump                                           // Metodo para determinar el id de la PDA en base a lo anterior
    )]
    pub registro: Account<'info, Registro>, // La PDA creada almacenara el Registro

    pub system_program: Program<'info, System>, // Programa necesario para crear la cuenta
}

// Contexto compartido por TODAS las instrucciones que leen o modifican el registro
#[derive(Accounts)]
pub struct ModificarVenta<'info> {
    pub owner: Signer<'info>, // Firma, pero no paga nada nuevo: por eso no lleva mut

    #[account(mut)]
    pub registro: Account<'info, Registro>, // mut porque se modifican los clientes y sus historiales
}
