import { useEffect, useRef, useState, type FormEvent } from 'react';
import { ArrowUpRight, Heart, Star, Zap } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { enable } from '@tauri-apps/plugin-autostart';

const playSound = (file: string) => {
  const audio = new Audio(`/sounds/${file}`);
  audio.volume = 0.25;

  audio.play().catch(() => {
    // Ignore autoplay/audio permission errors
  });
};

type Mood =
  | 'happy'
  | 'sleepy'
  | 'suspicious'
  | 'disappointed'
  | 'angry'
  | 'proud'
  | 'celebrating'
  | 'confused'
  | 'judging';

type Reaction = {
  label: string;
  mood: Mood;
  message: string;
  color: string;
};

type LastAction =
  | 'none'
  | 'studied'
  | 'procrastinating'
  | 'motivated'
  | 'judged'
  | 'gave-up'
  | 'chatted'
  | 'clicked';

const reactions: Reaction[] = [
  {
    label: 'I studied',
    mood: 'proud',
    message: 'OH??? LOOK WHO BECAME ACADEMICALLY RESPONSIBLE.',
    color: 'mint',
  },
  {
    label: "I'm procrastinating",
    mood: 'disappointed',
    message: 'Three. HOURS. Annie, be serious.',
    color: 'peach',
  },
  {
    label: 'Motivate me',
    mood: 'angry',
    message: 'OPEN. THE. BOOK. WE ARE NOT LOSING TO A CHAIR TODAY.',
    color: 'yellow',
  },
  {
    label: 'Judge me',
    mood: 'judging',
    message: 'I have reviewed the evidence. The vibes are criminal.',
    color: 'pink',
  },
  {
    label: 'I give up',
    mood: 'sleepy',
    message: 'Fine. Five minutes of dramatic floor time. Then we rise.',
    color: 'lavender',
  },
];

const starterMessage = 'hi. I am ZUZU. click me.';

const idleLines = [
  'you. stop scrolling.',
  'is that a phone I see.',
  'blink if you are still alive.',
  'I will judge later. I am resting.',
  'the chair is not winning today.',

  // mildly concerned ZUZU
  'are we studying or conducting a staring contest.',
  'just checking. are we doing anything.',
  'I have been supervising. you are welcome.',
  'interesting choice of activity.',
  'I am watching. respectfully.',

  // tiny menace
  'wow. absolutely nothing is happening.',
  'you opened your laptop. points for theatre.',
  'I know what you are doing.',
  'that tab is suspicious.',
  'you cannot fool me. I have eyes.',

  // cute
  'hi again.',
  'just wanted to say hi.',
  'you are doing okay. probably.',
  'I believe in you. unfortunately.',
  'tiny reminder: you got this.',

  // completely unnecessary
  'I have decided that today is Tuesday.',
  'do you think cats understand taxes.',
  'I forgot what I was going to say.',
  'never mind.',
  '...',
];

function App() {
    useEffect(() => {
    enable().catch((error) => {
      console.error('ZUZU autostart could not be enabled:', error);
    });
  }, []);
  const [mood, setMood] = useState<Mood>('happy');
  const [message, setMessage] = useState(starterMessage);
  const [input, setInput] = useState('');
  const [isTalking, setIsTalking] = useState(false);
  const [sparkles, setSparkles] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const [bubbleVisible, setBubbleVisible] = useState(true);
  const [lastAction, setLastAction] = useState<LastAction>('none');
  const [interactionCount, setInteractionCount] = useState(0);

  const stageRef = useRef<HTMLDivElement>(null);

  // Used to distinguish a click from a drag.
  const pointerStart = useRef({ x: 0, y: 0 });
  const isDragging = useRef(false);

  useEffect(() => {
    if (!isTalking) return;

    const timeout = window.setTimeout(() => {
      setIsTalking(false);
    }, 500);

    return () => window.clearTimeout(timeout);
  }, [isTalking]);

  useEffect(() => {
  if (menuOpen) return;

  const idleTimer = window.setInterval(() => {
    let next: string;
    let nextMood: Mood = 'happy';

    if (lastAction === 'studied') {
      const studiedLines = [
        'still proud of you, unfortunately.',
        'look at you being productive.',
        'I have decided to respect you today.',
        'okay academic weapon. calm down.',
      ];

      next =
        studiedLines[
          Math.floor(Math.random() * studiedLines.length)
        ];

      nextMood = 'proud';
    } else if (lastAction === 'procrastinating') {
      const procrastinationLines = [
        'so... are we still procrastinating.',
        'I remember what you admitted earlier.',
        'the evidence remains concerning.',
        'you said you were procrastinating. I have not forgotten.',
      ];

      next =
        procrastinationLines[
          Math.floor(
            Math.random() * procrastinationLines.length
          )
        ];

      nextMood = 'judging';
    } else if (lastAction === 'gave-up') {
      const gaveUpLines = [
        'you gave up remarkably quickly.',
        'dramatic floor time still available.',
        'I am allowing exactly five minutes of nonsense.',
        'we rise again eventually.',
      ];

      next =
        gaveUpLines[
          Math.floor(Math.random() * gaveUpLines.length)
        ];

      nextMood = 'sleepy';
    } else if (lastAction === 'motivated') {
      const motivatedLines = [
        'I expect results.',
        'go on. impress me.',
        'I am watching that book.',
        'no pressure. actually, some pressure.',
      ];

      next =
        motivatedLines[
          Math.floor(Math.random() * motivatedLines.length)
        ];

      nextMood = 'angry';
    } else if (lastAction === 'judged') {
      const judgedLines = [
        'you asked to be judged. I have not forgotten.',
        'the evidence is still under review.',
        'I am keeping my eye on you.',
        'interesting behaviour continues.',
      ];

      next =
        judgedLines[
          Math.floor(Math.random() * judgedLines.length)
        ];

      nextMood = 'judging';
    } else if (lastAction === 'clicked') {
      const clickedLines = [
        'you again.',
        'did you need something.',
        'yes, I am still here.',
        'you clicked me and then just stood there.',
      ];

      next =
        clickedLines[
          Math.floor(Math.random() * clickedLines.length)
        ];

      nextMood = 'suspicious';
    } else {
      next =
        idleLines[
          Math.floor(Math.random() * idleLines.length)
        ];

      nextMood =
        Math.random() < 0.7
          ? 'happy'
          : Math.random() < 0.5
            ? 'suspicious'
            : 'sleepy';
    }

    setMood(nextMood);
    setMessage(next);
    setIsTalking(true);
    setBubbleVisible(true);
    playSound('zuzu-blip.mp3');
  }, 20000);

  return () => window.clearInterval(idleTimer);
}, [menuOpen, lastAction]);

  useEffect(() => {
    if (!menuOpen) return;

    const handleClick = (event: MouseEvent) => {
      if (
        stageRef.current &&
        !stageRef.current.contains(event.target as Node)
      ) {
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

  setSparkles(
    reaction.mood === 'proud' ||
      reaction.mood === 'celebrating'
  );

  setMenuOpen(false);
  setBubbleVisible(true);

  setInteractionCount((count) => count + 1);

  if (reaction.label === 'I studied') {
    setLastAction('studied');
    playSound('zuzu-happy.mp3');
  } else if (reaction.label === "I'm procrastinating") {
    setLastAction('procrastinating');
  } else if (reaction.label === 'Motivate me') {
    setLastAction('motivated');
  } else if (reaction.label === 'Judge me') {
    setLastAction('judged');
  } else if (reaction.label === 'I give up') {
    setLastAction('gave-up');
  }
};

  const sendMessage = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const cleanInput = input.trim();

    if (!cleanInput) return;

    const lowerInput = cleanInput.toLowerCase();

    const matchedReaction =
      lowerInput.includes('stud') ||
      lowerInput.includes('finish')
        ? {
            mood: 'celebrating' as Mood,
            message:
              'YOU FINISHED? I am putting this in the history books.',
          }
        : lowerInput.includes('later') ||
            lowerInput.includes('tomorrow')
          ? {
              mood: 'suspicious' as Mood,
              message:
                'Sure. And I am the Prime Minister.',
            }
          : lowerInput.includes('procrast') ||
              lowerInput.includes('nothing')
            ? {
                mood: 'disappointed' as Mood,
                message:
                  'Bold of you to announce that to the witness.',
              }
            : {
                mood: 'confused' as Mood,
                message:
                  'Interesting. Concerning. But interesting.',
              };

    setMood(matchedReaction.mood);
    setMessage(matchedReaction.message);
    setInput('');
    setIsTalking(true);
    setSparkles(
      matchedReaction.mood === 'celebrating'
    );
    setBubbleVisible(true);
  };

  const toggleMenu = () => {
  if (isDragging.current) {
    isDragging.current = false;
    return;
  }

  setInteractionCount((count) => count + 1);
  setLastAction('clicked');
  playSound('zuzu-click.mp3');

  if (!menuOpen) {
    setMood('happy');
    setMessage('oh. you remembered I exist.');
    setIsTalking(true);
    setBubbleVisible(true);

    window.setTimeout(() => {
      setBubbleVisible(false);
      setMenuOpen(true);
    }, 700);

    return;
  }

  setMenuOpen(false);
  setBubbleVisible(true);
};

  const handleMouseDown = async (
    event: React.MouseEvent<HTMLButtonElement>
  ) => {
    if (event.button !== 0) return;

    pointerStart.current = {
      x: event.clientX,
      y: event.clientY,
    };

    isDragging.current = false;

    const handleMouseMove = async (moveEvent: MouseEvent) => {
      const dx =
        moveEvent.clientX - pointerStart.current.x;

      const dy =
        moveEvent.clientY - pointerStart.current.y;

      const distance = Math.sqrt(dx * dx + dy * dy);

      // Small movements are treated as clicks.
      if (distance < 6) return;

      isDragging.current = true;

      window.removeEventListener(
        'mousemove',
        handleMouseMove
      );

      window.removeEventListener(
        'mouseup',
        handleMouseUp
      );

      try {
        await getCurrentWindow().startDragging();
      } catch (error) {
        console.error(
          'ZUZU could not be dragged:',
          error
        );
      }
    };

    const handleMouseUp = () => {
      window.removeEventListener(
        'mousemove',
        handleMouseMove
      );

      window.removeEventListener(
        'mouseup',
        handleMouseUp
      );
    };

    window.addEventListener(
      'mousemove',
      handleMouseMove
    );

    window.addEventListener(
      'mouseup',
      handleMouseUp
    );
  };

  return (
    <main className="desktop-pet-stage">
      <div
        className="scanlines"
        aria-hidden="true"
      />

      <div
        ref={stageRef}
        className={`pet-zuzu ${
          menuOpen ? 'menu-open' : ''
        }`}
      >
        <button
          className="pet-button"
          onMouseDown={handleMouseDown}
          onClick={toggleMenu}
          aria-label="ZUZU — click to interact"
          aria-expanded={menuOpen}
        >
          <div
            className={`pet-sprite mood-${mood} ${
              isTalking ? 'talking' : ''
            }`}
          >
            <div
              className="pixel-cat"
              role="img"
              aria-label="ZUZU, a black pixel cat companion"
            >
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

            <div
              className="pixel-hearts"
              aria-hidden="true"
            >
              <Heart
                size={14}
                fill="currentColor"
              />
              <Heart
                size={9}
                fill="currentColor"
              />
            </div>
          </div>
        </button>

        {bubbleVisible && !menuOpen && (
          <div className="speech-bubble-wrap">
            <div
              className={`speech-bubble ${
                isTalking ? 'talking' : ''
              }`}
            >
              <span className="bubble-tail" />

              <span className="bubble-label">
                ZUZU SAYS
              </span>

              <p>{message}</p>

              <div className="bubble-dots">
                •••
              </div>
            </div>
          </div>
        )}

        {menuOpen && (
          <div className="pet-menu">
            <div className="menu-title">
              tell ZUZU what’s up
            </div>

            <div className="menu-reactions">
              {reactions.map((reaction) => (
                <button
                  key={reaction.label}
                  className={`menu-button ${reaction.color}`}
                  onClick={() =>
                    triggerReaction(reaction)
                  }
                >
                  <span className="button-icon">
                    {reaction.mood === 'proud' ? (
                      <Star
                        size={14}
                        fill="currentColor"
                      />
                    ) : reaction.mood === 'angry' ? (
                      <Zap
                        size={14}
                        fill="currentColor"
                      />
                    ) : (
                      <Heart
                        size={14}
                        fill="currentColor"
                      />
                    )}
                  </span>

                  {reaction.label}
                </button>
              ))}
            </div>

            <form
              className="menu-form"
              onSubmit={sendMessage}
            >
              <input
                value={input}
                onChange={(event) =>
                  setInput(event.target.value)
                }
                placeholder="say something..."
                aria-label="Say something to ZUZU"
              />

              <button
                type="submit"
                className="send-button"
                aria-label="Send"
              >
                <ArrowUpRight size={16} />
              </button>
            </form>
          </div>
        )}
      </div>

      {sparkles && (
        <div
          className="celebration-layer"
          aria-hidden="true"
        >
          <span>✦</span>
          <span>♡</span>
          <span>✧</span>
          <span>★</span>
          <span>✦</span>
        </div>
      )}
    </main>
  );
}

export default App;