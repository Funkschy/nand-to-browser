\NeedsTeXFormat{LaTeX2e}
\ProvidesClass{hhuthesis}[2019/10/13 HHU Thesis LaTeX document class v1.6.1]

\DeclareOption*{\PassOptionsToClass{\CurrentOption}{article}} % Options fallback.
\ProcessOptions\relax

\LoadClass[11pt,a4paper,twoside,titlepage]{article}


%%% Define user-settings commands

%% Cannot use \startdate somehow
\newcommand{\begindate}[1]{\renewcommand{\@begin}{#1}}
\newcommand{\@begin}{}
\newcommand{\duedate}[1]{\renewcommand{\@due}{#1}}
\newcommand{\@due}{}

\newcommand{\firstexaminer}[1]{\renewcommand{\@fstexaminer}{#1}}
\newcommand{\@fstexaminer}{}
\newcommand{\secondexaminer}[1]{\renewcommand{\@sndexaminer}{#1}}
\newcommand{\@sndexaminer}{}

\newcommand{\gratuationtype}[1]{\renewcommand{\@degree}{#1}}
\newcommand{\@degree}{Bachelor}

%% Allows switching between colour and black/white printing
\newif\ifprintblackwhite
\printblackwhitefalse

\newcommand{\blackwhiteprint}{
  \printblackwhitetrue
  \pgfplotsset{cycle list name=hhubwcycle}
}


%%% Necessary packages

%% Allows direct insertion of umlauts (ä, ö, ü instead of \"a, \"o, \"u)
\RequirePackage[utf8]{inputenc}
\RequirePackage[T1]{fontenc}

\RequirePackage[nottoc]{tocbibind}
\RequirePackage{iflang} % Needed for language dependend distinctions

\RequirePackage{xcolor}
%% HHU Corporate Design colours
\definecolor{hhublue}{HTML}{006ab3} % Blau
\definecolor{hhudarkblue}{HTML}{003964} % Dunkelblau
\definecolor{hhuiceblue}{HTML}{b5cbd6} % Eisblau
\definecolor{hhucyan}{HTML}{57bab1} % Türkis
\definecolor{hhugreen}{HTML}{8cb110} % Grün
\definecolor{hhuorange}{HTML}{ee7f00} % Orange
\definecolor{hhured}{HTML}{be0a26} % Rot

\RequirePackage{tikz}
\RequirePackage{pgf,pgfplots}
%% Setup of colour cylce lists for pgfplots
\pgfplotscreateplotcyclelist{hhucolorcycle}{
  solid,hhublue,every mark/.append style={solid},mark=*\\
  dashed,hhured,every mark/.append style={solid},mark=square*\\
  dashdotted,hhugreen,every mark/.append style={solid},mark=diamond*\\
  dotted,hhuorange,every mark/.append style={solid},mark=triangle*\\
  densely dashed,hhucyan,every mark/.append style={solid},mark=star\\
  loosely dashed,hhublue,every mark/.append style={solid},mark=o\\
  loosely dotted,hhured,every mark/.append style={solid},mark=square\\
  densely dashed,hhugreen,every mark/.append style={solid},mark=diamond\\
  loosely dashdotted,hhuorange,every mark/.append style={solid},mark=triangle\\
  densely dotted,hhucyan,every mark/.append style={solid},mark=star\\
}
\pgfplotscreateplotcyclelist{hhubwcycle}{ % For black/white print
  solid,every mark/.append style={solid,fill=gray},mark=*\\
  dashed,every mark/.append style={solid,fill=gray},mark=square*\\
  dashdotted,every mark/.append style={solid,fill=gray},mark=diamond*\\
  dotted,every mark/.append style={solid,fill=gray},mark=triangle*\\
  densely dashed,every mark/.append style={solid},mark=star\\
  loosely dashed,every mark/.append style={solid},mark=o\\
  loosely dotted,every mark/.append style={solid},mark=square\\
  densely dashed,every mark/.append style={solid},mark=diamond\\
  loosely dashdotted,every mark/.append style={solid},mark=triangle\\
  densely dotted,every mark/.append style={solid},mark=star\\
}
%% Default colour cycle list
%% The colour cycle lists are further defined below.
%% Note that calling \blackwhiteprint switches this to hhubwcycle.
\pgfplotsset{cycle list name=hhucolorcycle}



%%% Formatting and Typesetting

%% Corporate Design says to use Arial, but TeXLive only provides helvet
\RequirePackage{helvet}
\renewcommand{\familydefault}{\sfdefault}

\pagestyle{headings}

\setlength{\textwidth}{15cm}

\setlength{\oddsidemargin}{35mm}
\setlength{\evensidemargin}{25mm}

\setlength{\baselineskip}{15pt}

\addtolength{\oddsidemargin}{-1in}
\addtolength{\evensidemargin}{-1in}

\parindent0mm
\parskip9pt

\RequirePackage{caption}
\captionsetup{width=0.9\textwidth,labelfont=bf,font=small}

%% Allow use of \frontmatter and \mainmatter
\newcommand{\frontmatter}{
  \cleardoublepage
  \pagenumbering{roman}
}

\newcommand{\mainmatter}{
  \cleardoublepage
  \pagenumbering{arabic}
}

\renewcommand{\appendix}{
  \clearpage
  {\LARGE\bf \appendixpagename}%
  \setcounter{section}{0}%
  \setcounter{subsection}{0}%
  \gdef\thesection{\@Alph\c@section}
  \let\section\appendix@section
}
\newcommand{\appendixpagename}{\iflanguage{ngerman}{Anhang}{Appendices}}
\newcommand\appendix@section[1]{%
  \refstepcounter{section}%
  \orig@section*{\appendixname{}~\thesection\hspace{1em}#1}%
  \addcontentsline{toc}{section}{\appendixname{}~\thesection\hspace{1em}#1}%
}
\let\orig@section\section

\newcommand{\backmatter}{
  \clearpage
  \let\section\orig@section
  \setcounter{secnumdepth}{0}
}

%%% Setup of title page

\RequirePackage{graphicx}

\newcommand{\@makeheader}{
  %% "Header"
  \vspace*{-5.46cm}
  \hspace{-10.4mm}
  \raisebox{-12mm}{
    \begin{minipage}[b]{70mm}
      \ifprintblackwhite
        \includegraphics[width=70mm]{fig/hhu-logo-black}%
      \else
        \includegraphics[width=70mm]{fig/hhu-logo-rgb}%
      \fi
    \end{minipage}%
  }%
  \hfill%
  \begin{minipage}[c]{6.8cm}
    \begin{center}
      {\large \textsc{Institut f\"ur Informatik \linebreak}}
      {\small\centering Softwaretechnik und Programmiersprachen\linebreak}
      {\scriptsize Universit\"atsstr. 1 \hspace{7mm} D--40225 D\"usseldorf}
    \end{center}
  \end{minipage}
}

\newcommand\@maketitle{%
  \vspace{3cm}
  \begin{center}
    {\LARGE \textbf \@title \par}%
    \vfill
    {\large \textbf \@author \par}%
    \vfill
    {\large \@degree{}arbeit \par}%
    \vspace{35mm}
    \begin{tabular}{ll}
      Beginn der Arbeit: & \@begin \\
      Abgabe der Arbeit: & \@due \\
      Gutachter:         & \@fstexaminer \\
                         & \@sndexaminer \\
    \end{tabular}
  \end{center}
}

\if@titlepage
  \renewcommand\maketitle{
    \begin{titlepage}%
      \let\footnotesize\small
      \let\footnoterule\relax
      \let\footnote\thanks
      \null\vfil
      \vskip 60\p@
      \@makeheader
      \@maketitle
      \@thanks
      \vfil\null
      \thispagestyle{empty}
    \end{titlepage}%
    \setcounter{footnote}{0}%
    % \global\let\thanks\relax
    % \global\let\maketitle\relax
    % \global\let\@thanks\@empty
    % \global\let\@date\@empty
    % \global\let\@title\@empty
    % \global\let\title\relax
    % \global\let\author\relax
    % \global\let\date\relax
    % \global\let\and\relax
    % \global\let\@begin\relax
    % \global\let\@due\relax
    % \global\let\@fstexaminer\relax
    % \global\let\@sndexaminer\relax
  }
\else
  \renewcommand\maketitle{\par
    \@makeheader
    \begingroup
      \renewcommand\thefootnote{\@fnsymbol\c@footnote}%
      \def\@makefnmark{\rlap{\@textsuperscript{\normalfont\@thefnmark}}}%
      \long\def\@makefntext##1{\parindent 1em\noindent
              \hb@xt@1.8em{%
                  \hss\@textsuperscript{\normalfont\@thefnmark}}##1}%
      \if@twocolumn
        \ifnum \col@number=\@ne
          \@maketitle
        \else
          \twocolumn[\@maketitle]%
        \fi
      \else
        \newpage
        \global\@topnum\z@   % Prevents figures from going at top of page.
        \@maketitle
      \fi
      \thispagestyle{plain}\@thanks
    \endgroup
    \setcounter{footnote}{0}%
    % \global\let\thanks\relax
    % \global\let\maketitle\relax
    % \global\let\@maketitle\relax
    % \global\let\@thanks\@empty
    % \global\let\@date\@empty
    % \global\let\@title\@empty
    % \global\let\title\relax
    % \global\let\author\relax
    % \global\let\date\relax
    % \global\let\and\relax
    % \global\let\@begin\relax
    % \global\let\@due\relax
    % \global\let\@fstexaminer\relax
    % \global\let\@sndexaminer\relax
  }
\fi

%% Renew titlepage to not reset page counter and to not fiddle with page style
\if@compatibility
\renewenvironment{titlepage}
    {%
      \if@twocolumn
        \@restonecoltrue\onecolumn
      \else
        \@restonecolfalse\newpage
      \fi
    }%
    {\if@restonecol\twocolumn \else \newpage \fi
    }
\else
\renewenvironment{titlepage}
    {%
      \if@twocolumn
        \@restonecoltrue\onecolumn
      \else
        \@restonecolfalse\newpage
      \fi
    }%
    {\if@restonecol\twocolumn \else \newpage \fi
     \if@twoside\else
        % \setcounter{page}\@ne
     \fi
    }
\fi

%% Redesign Abstract
\if@titlepage
  \renewenvironment{abstract}{%
      \titlepage
      \@beginparpenalty\@lowpenalty
      \begin{center}%
        \bfseries \large \abstractname
        \@endparpenalty\@M
      \end{center}}%
     {\par\endtitlepage}
\else
  \renewenvironment{abstract}{%
      \if@twocolumn
        \section*{\abstractname}%
      \else
        \small
        \begin{center}%
          {\bfseries \large \abstractname\vspace{-.5em}\vspace{\z@}}%
        \end{center}%
        \quotation
      \fi}
      {\if@twocolumn\else\endquotation\fi}
\fi

%% Design official statement of independend and autonomous work
\newcommand{\@makedeclaration}{%
  \@beginparpenalty\@lowpenalty
  \begin{center}%
    \bfseries \large Ehrenw\"ortliche Erkl\"arung
    \@endparpenalty\@M
  \end{center}%

  Hiermit versichere ich
  die vorliegende \@degree{}arbeit selbstst\"andig verfasst und keine anderen
  als die angegebenen Quellen und Hilfsmittel benutzt zu haben.
  Alle Stellen, die aus den Quellen entnommen wurden,
  sind als solche kenntlich gemacht worden.
  Diese Arbeit hat in gleicher oder \"ahnlicher Form
  noch keiner Pr\"ufungsbeh\"orde vorgelegen.

  \vspace{2cm}

  \begin{centering}
    \begin{tabular}{lc}
    D\"usseldorf, den \@due \hspace*{1.8cm} & \underline{\hspace{6cm}}\\
    & \@author
    \end{tabular}
  \end{centering}
}

\if@titlepage
  \newcommand\makedeclaration{
    \begin{titlepage}%
      \null\vfil
      \vskip 60\p@
      \@makedeclaration
      \vfil\null
    \end{titlepage}%
  }
\else
  \newcommand\makedeclaration{\par
    \begingroup
      \renewcommand\thefootnote{\@fnsymbol\c@footnote}%
      \def\@makefnmark{\rlap{\@textsuperscript{\normalfont\@thefnmark}}}%
      \long\def\@makefntext##1{\parindent 1em\noindent
              \hb@xt@1.8em{%
                  \hss\@textsuperscript{\normalfont\@thefnmark}}##1}%
      \if@twocolumn
        \ifnum \col@number=\@ne
          \@makedeclaration
        \else
          \twocolumn[\@makedeclaration]%
        \fi
      \else
        \newpage
        \global\@topnum\z@   % Prevents figures from going at top of page.
        \@makedeclaration
      \fi
      \thispagestyle{plain}\@thanks
    \endgroup
  }
\fi

%% design Acknowledgements
\newcommand{\acknowledgementsname}{%
  \iflanguage{ngerman}{Danksagung}{Acknowledgements}%
}
\if@titlepage
  \newenvironment{acknowledgements}{%
      \titlepage
      \@beginparpenalty\@lowpenalty
      \begin{center}%
        \bfseries \large \acknowledgementsname
        \@endparpenalty\@M
      \end{center}}%
     {\par\endtitlepage\cleardoublepage}
\else
  \newenvironment{acknowledgements}{%
      \if@twocolumn
        \section*{\acknowledgementsname}%
      \else
        \small
        \begin{center}%
          {\bfseries \large \abstractname\vspace{-.5em}\vspace{\z@}}%
        \end{center}%
        \quotation
      \fi}
      {\if@twocolumn\else\endquotation\fi\cleardoublepage}
\fi


%%% Simple command to set up frontmatter

\newcommand{\makefrontmatter}{
  \maketitle
  \cleardoublepage

  \makedeclaration % Declaration of Authorship
  \cleardoublepage

  \begin{abstract}
    \input{abstract.tex}
  \end{abstract}
  \cleardoublepage
}
