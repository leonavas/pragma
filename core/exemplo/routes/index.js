export const state = ({ shared }) => ({
  get tomTarefas() { return shared.tarefas.pendentes ? 'neutro' : 'alerta' },
  get tomCarrinho() { return shared.carrinho.vazio ? 'alerta' : 'ok' },
})
