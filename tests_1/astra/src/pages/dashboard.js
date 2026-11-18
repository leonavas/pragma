import { component } from '../lib/component.js';
import { Counter, StatCard, Clock, ThemeToggle, Tabs, Modal, paragraph } from '../components/common.js';
import { pendingCount, cartCount, cartTotal, money } from '../state.js';
export const Dashboard = {
  mount(root, props) {
    const c = component(root, '<div class="page-heading"><div><h1>Seu dia, em ordem.</h1><p class="muted">Tarefas organizadas. Compras no ponto.</p></div><div data-clock></div></div><div class="stats"><div data-pending></div><div data-items></div><div data-total></div></div><div class="dashboard-columns"><section class="surface"><h2>Um passo de cada vez</h2><p class="muted">Contadores independentes para acompanhar seu ritmo.</p><div data-count-a></div><div data-count-b></div></section><section class="surface"><h2>Do seu jeito</h2><p class="muted">Escolha o tema mais confortável para agora.</p><div class="preferences"><div data-theme></div><div data-modal></div></div></section></div><section class="surface" data-tabs></section>');
    c.own(Clock.mount(c.find('[data-clock]')));
    c.own(Counter.mount(c.find('[data-count-a]'), { label: 'Cliques', initial: 0, step: 1 }));
    c.own(Counter.mount(c.find('[data-count-b]'), { label: 'Pulos de 5', initial: 10, step: 5, min: 0, max: 50 }));
    const theme = c.own(ThemeToggle.mount(c.find('[data-theme]'), props.state));
    const stats = [
      c.own(StatCard.mount(c.find('[data-pending]'), {})),
      c.own(StatCard.mount(c.find('[data-items]'), {})),
      c.own(StatCard.mount(c.find('[data-total]'), {})),
    ];
    c.own(Tabs.mount(c.find('[data-tabs]'), { items: [
      { label: 'Sobre', mount: paragraph('Mercadinho reúne suas tarefas e compras em um lugar simples. Suas escolhas ficam salvas neste navegador.') },
      { label: 'Atalhos', mount: paragraph('Use as setas para mudar de aba. Nas tarefas, pressione Enter para salvar uma edição e Esc para cancelar.') },
      { label: 'Contador', mount: root => Counter.mount(root, { label: 'Na aba', initial: 3, step: 3 }) },
    ] }));
    c.own(Modal.mount(c.find('[data-modal]'), {
      trigger: 'Sobre o app', title: 'Um lugar para o cotidiano',
      content: paragraph('O Mercadinho ajuda você a organizar tarefas e montar sua lista de compras. Os dados são salvos apenas neste navegador.'),
    }));
    function update(p) {
      const pending = pendingCount(p.state);
      const items = cartCount(p.state);
      stats[0].update({ title: 'Tarefas pendentes', value: pending, hint: 'O que ainda precisa da sua atenção', tone: pending ? 'neutro' : 'alerta' });
      stats[1].update({ title: 'Itens no carrinho', value: items, hint: 'Sua próxima compra', tone: items ? 'neutro' : 'alerta' });
      stats[2].update({ title: 'Total do carrinho', value: money(cartTotal(p.state)), hint: 'Somando todos os itens', tone: items ? 'ok' : 'alerta' });
      theme.update(p.state);
    }
    update(props);
    return { update, destroy: c.destroy };
  },
};
