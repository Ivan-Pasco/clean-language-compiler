/-
  Clean Language — piloto. Lo que puede salir de tipar o de evaluar.

  Cuatro salidas, y las dos últimas son la medida del piloto:
  `sinEspecificar` es un caso que la ley no decide; `fueraDeAlcance`
  es un caso que la ley decide y el piloto no cubre.
-/
namespace Clean

inductive Resultado (α : Type) where
  /-- La ley da este resultado. -/
  | bien (v : α)
  /-- La ley da un diagnóstico con código: SEM001, RUN003, … -/
  | diagnostico (codigo : String) (mensaje : String)
  /-- La ley no dice qué pasa aquí. -/
  | sinEspecificar (que : String)
  /-- La ley lo dice; el piloto no lo cubre. -/
  | fueraDeAlcance (que : String)
  deriving Inhabited

namespace Resultado

def bind {α β : Type} (r : Resultado α) (f : α → Resultado β) : Resultado β :=
  match r with
  | .bien v => f v
  | .diagnostico c m => .diagnostico c m
  | .sinEspecificar q => .sinEspecificar q
  | .fueraDeAlcance q => .fueraDeAlcance q

instance : Monad Resultado where
  pure := .bien
  bind := bind

end Resultado

end Clean
