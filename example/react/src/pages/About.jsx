function About() {
  return (
    <div className="page-container">
      <div className="page-header">
        <h1 className="page-title">Sobre Nós</h1>
        <p className="page-subtitle">
          Saiba mais sobre este projeto de demonstração e como as Single Page Applications 
          mudam a forma como interagimos com a web.
        </p>
      </div>

      <div className="card" style={{ maxWidth: '800px', margin: '0 auto', textAlign: 'center', padding: '3rem' }}>
        <h2 style={{ fontSize: '2rem', marginBottom: '1.5rem', fontWeight: 700 }}>O Poder das SPAs</h2>
        <p style={{ fontSize: '1.1rem', marginBottom: '1.5rem' }}>
          Em uma Single Page Application, apenas uma página web é carregada inicialmente. Quando 
          o usuário navega pelo site, o conteúdo da página é atualizado dinamicamente usando JavaScript.
        </p>
        <p style={{ fontSize: '1.1rem', marginBottom: '2rem' }}>
          Isso cria uma experiência muito mais rápida e próxima de um aplicativo nativo, pois evita
          o carregamento completo de páginas em cada clique.
        </p>
        
        <div style={{ display: 'inline-block', background: 'rgba(99, 102, 241, 0.1)', padding: '1rem 2rem', borderRadius: '12px', border: '1px solid rgba(99, 102, 241, 0.3)' }}>
          <strong style={{ color: 'var(--primary)' }}>Versão do Projeto:</strong> 1.0.0 Alpha
        </div>
      </div>
    </div>
  );
}

export default About;
