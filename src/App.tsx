import { useEffect, useRef, useState, type FormEvent } from 'react';
import { ArrowUpRight, Heart, Star, Zap } from 'lucide-react';

type Mood = 'happy' | 'sleepy' | 'suspicious' | 'disappointed' | 'angry' | 'proud' | 'celebrating' | 'confused' | 'judging';

type Reaction = {
  label: string;
  mood: Mood;
  message: string;
  color: string;
};

const reactions: Reaction[] = [
  { label: 'I studied', mood: 'proud', message: 'OH??? LOOK WHO BECAME ACADEMICALLY RESPONSIBLE.', color: 'mint' },
  { label: "I'm procrastinating", mood: 'disappointed', message: 'Three. HOURS. Annie, be serious.', color: 'peach' },
  { label: 'Motivate me', mood: 'angry', message: 'OPEN. THE. BOOK. WE ARE NOT LOSING TO A CHAIR TODAY.', color: 'yellow' },
  { label: 'Judge me', mood: 'judging', message: 'I have reviewed the evidence. The vibes are criminal.', color: 'pink' },
  { label: 'I give up', mood: 'sleepy', message: 'Fine. Five minutes of dramatic floor time. Then we rise.', color: 'lavender' },
];

const starterMessage = 'hi. I am ZUZU. click me.';
const idleLines = [
  'you. stop scrolling.',
  'is that a phone I see.',
  'blink if you are still alive.',
  'I will judge later. I am resting.',
  'the chair is not winning today.',
];

function App() {
  const [mood, setMood] = useState<Mood>('happy');
  const [message, setMessage] = useState(starterMessage);
  const [input, setInput] = useState('');
  const [isTalking, setIsTalking] = useState(false);
  const [sparkles, setSparkles] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const [bubbleVisible, setBubbleVisible] = useState(true);
  const stageRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isTalking) return;
    const timeout = window.setTimeout(() => setIsTalking(false), 500);
    return () => window.clearTimeout(timeout);
  }, [isTalking]);

  useEffect(() => {
    if (menuOpen) return;
    const idleTimer = window.setInterval(() => {
      const next = idleLines[Math.floor(Math.random() * idleLines.length)];
      setMessage(next);
      setBubbleVisible(true);
    }, 14000);
    return () => window.clearInterval(idleTimer);
  }, [menuOpen]);

  useEffect(() => {
    if (!menuOpen) return;
    const handleClick = (event: MouseEvent) => {
      if (stageRef.current && !stageRef.current.contains(event.target as Node)) {
        setMenuOpen(false);
      }
    };
    window.addEventListener('mousedown', handleClick);
    return () => window.removeEventListener('mousedown', handleClick);
  }, [menuOpen]);

  const triggerReaction = (reaction: Reaction) => {
    setMood(reaction.mood);
    setMessage(reaction.message);
    setIsTalking(true);
    setSparkles(reaction.mood === 'proud' || reaction.mood === 'celebrating');
    setMenuOpen(false);
    setBubbleVisible(true);
  };

  const sendMessage = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const cleanInput = input.trim();
    if (!cleanInput) return;
    const lowerInput = cleanInput.toLowerCase();
    const matchedReaction = lowerInput.includes('stud') || lowerInput.includes('finish')
      ? { mood: 'celebrating' as Mood, message: 'YOU FINISHED? I am putting this in the history books.' }
      : lowerInput.includes('later') || lowerInput.includes('tomorrow')
        ? { mood: 'suspicious' as Mood, message: 'Sure. And I am the Prime Minister.' }
        : lowerInput.includes('procrast') || lowerInput.includes('nothing')
          ? { mood: 'disappointed' as Mood, message: 'Bold of you to announce that to the witness.', }
          : { mood: 'confused' as Mood, message: 'Interesting. Concerning. But interesting.' };

    setMood(matchedReaction.mood);
    setMessage(matchedReaction.message);
    setInput('');
    setIsTalking(true);
    setSparkles(matchedReaction.mood === 'celebrating');
    setBubbleVisible(true);
  };

  const toggleMenu = () => {
    setMenuOpen((open) => !open);
    if (!menuOpen) setBubbleVisible(false);
  };

  return (
    <main className="desktop-pet-stage">
      <div className="scanlines" aria-hidden="true" />

      <div ref={stageRef} className={`pet-zuzu ${menuOpen ? 'menu-open' : ''}`}>
        <button
          className="pet-button"
          onClick={toggleMenu}
          aria-label="ZUZU — click to interact"
          aria-expanded={menuOpen}
        >
          <div className={`pet-sprite mood-${mood} ${isTalking ? 'talking' : ''}`}>
            <div className="pixel-cat" role="img" aria-label="ZUZU, a black pixel cat companion">
              <span className="cat-ear ear-left" />
              <span className="cat-ear ear-right" />
              <span className="cat-head">
                <span className="cat-eye eye-left" />
                <span className="cat-eye eye-right" />
                <span className="cat-nose" />
                <span className="cat-mouth" />
              </span>
              <span className="cat-body" />
              <span className="cat-tail" />
            </div>
            <div className="pet-shadow" />
            <div className="pixel-hearts" aria-hidden="true"><Heart size={14} fill="currentColor" /><Heart size={9} fill="currentColor" /></div>
          </div>
        </button>

        {bubbleVisible && !menuOpen && (
          <div className="speech-bubble-wrap">
            <div className={`speech-bubble ${isTalking ? 'talking' : ''}`}>
              <span className="bubble-tail" />
              <span className="bubble-label">ZUZU SAYS</span>
              <p>{message}</p>
              <div className="bubble-dots">•••</div>
            </div>
          </div>
        )}

        {menuOpen && (
          <div className="pet-menu">
            <div className="menu-title">tell ZUZU what’s up</div>
            <div className="menu-reactions">
              {reactions.map((reaction) => (
                <button
                  key={reaction.label}
                  className={`menu-button ${reaction.color}`}
                  onClick={() => triggerReaction(reaction)}
                >
                  <span className="button-icon">
                    {reaction.mood === 'proud'
                      ? <Star size={14} fill="currentColor" />
                      : reaction.mood === 'angry'
                        ? <Zap size={14} fill="currentColor" />
                        : <Heart size={14} fill="currentColor" />}
                  </span>
                  {reaction.label}
                </button>
              ))}
            </div>
            <form className="menu-form" onSubmit={sendMessage}>
              <input
                value={input}
                onChange={(event) => setInput(event.target.value)}
                placeholder="say something..."
                aria-label="Say something to ZUZU"
              />
              <button type="submit" className="send-button" aria-label="Send">
                <ArrowUpRight size={16} />
              </button>
            </form>
          </div>
        )}
      </div>

      {sparkles && (
        <div className="celebration-layer" aria-hidden="true">
          <span>✦</span><span>♡</span><span>✧</span><span>★</span><span>✦</span>
        </div>
      )}
    </main>
  );
}

export default App;
