function Home() {
  return (
    <div className="page-container">
      <div className="page-header">
        <h1 className="page-title">Bem-vindo ao Home</h1>
        <p className="page-subtitle">
          Esta é a página principal da sua nova aplicação React. Descubra a performance
          e a fluidez de uma Single Page Application.
        </p>
      </div>

      <div className="cards-grid">
        <div className="card">
          <div className="card-icon">🚀</div>
          <h3>Performance</h3>
          <p>Navegação instantânea entre as páginas sem recarregar o navegador. A experiência é muito mais fluida.</p>
        </div>
        
        <div className="card">
          <div className="card-icon">🎨</div>
          <h3>Design Moderno</h3>
          <p>Interface clean com modo escuro, efeitos de vidro e tipografia cuidadosamente selecionada para agradar aos olhos.</p>
        </div>

        <div className="card">
          <div className="card-icon">⚡</div>
          <h3>Vite + React</h3>
          <p>Construído utilizando o que há de melhor no ecossistema atual para o desenvolvimento frontend.</p>
        </div>
      </div>
      
      <div style={{ textAlign: 'center', marginTop: '3rem' }}>
        <button className="btn btn-primary" onClick={() => alert('Pronto para decolar!')}>
          Começar Agora
        </button>
      </div>
    </div>
  );
}

export default Home;
