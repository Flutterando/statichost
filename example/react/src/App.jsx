import { Routes, Route, Link, useLocation } from 'react-router-dom';
import Home from './pages/Home';
import About from './pages/About';

function App() {
  const location = useLocation();

  return (
    <div className="app-container">
      <nav className="navbar">
        <div className="logo">React<span>App</span></div>
        <div className="nav-links">
          <Link to="/" className={`nav-link ${location.pathname === '/' ? 'active' : ''}`}>Home</Link>
          <Link to="/about" className={`nav-link ${location.pathname === '/about' ? 'active' : ''}`}>Sobre</Link>
        </div>
      </nav>

      <main className="main-content">
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/about" element={<About />} />
        </Routes>
      </main>

      <footer className="footer">
        <p>&copy; 2026 React SPA Demo. Todos os direitos reservados.</p>
      </footer>
    </div>
  );
}

export default App;
